use eframe::{App, CreationContext};
use egui::{Color32, ComboBox, Grid, RichText, ScrollArea, Ui};
use egui_plot::{Bar, BarChart, Plot};
use poll_promise::Promise;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tel_core::models::Token;

// For direct database access
use rusqlite::Connection;
use std::path::Path;

// API endpoints
const API_BASE_URL: &str = "http://127.0.0.1:8081";
const DEFAULT_DB_PATH: &str = "sqlite_tel_on_chain.db";

// Type aliases from the main project to use with the API
type Address = alloy_primitives::Address;

#[derive(Debug, Clone, Deserialize)]
struct LiquidityWall {
    price_lower: f64,
    price_upper: f64,
    liquidity_value: f64,
    dex_sources: HashMap<String, f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct LiquidityWallsResponse {
    token0: Token,
    token1: Token,
    price: f64,
    buy_walls: Vec<LiquidityWall>,
    sell_walls: Vec<LiquidityWall>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

// Database query results
#[derive(Debug, Clone)]
pub struct DbPool {
    address: String,
    dex: String,
    chain_id: u64,
    token0: String,
    token1: String,
    fee: u32,
}

#[derive(Debug, Clone)]
struct DbToken {
    address: String,
    symbol: String,
    name: String,
    decimals: u8,
    chain_id: u64,
}

#[derive(Debug, Clone)]
struct DbLiquidityDistribution {
    pool_address: String,
    token0_address: String,
    token1_address: String,
    timestamp: i64,
    price_points: usize,
}

#[derive(Default)]
pub struct TelOnChainUI {
    // API connection state
    api_status: String,
    selected_dex: String,
    available_dexes: Vec<String>,
    selected_chain_id: u64,
    available_chain_ids: Vec<u64>,

    // Token selection
    token0_address: String,
    token1_address: String,
    available_tokens: HashMap<u64, Vec<String>>, // chain_id -> token symbols

    // API response data
    liquidity_data: Option<Arc<LiquidityWallsResponse>>,
    liquidity_promise: Option<Promise<Result<LiquidityWallsResponse, String>>>,

    // Database access
    db_path: String,
    db_pools: Vec<DbPool>,
    db_tokens: Vec<DbToken>,
    db_distributions: Vec<DbLiquidityDistribution>,
    db_query_status: String,

    // UI tabs
    selected_tab: Tab,

    // Pool-Info tab state
    selected_pool_idx: Option<usize>,
    pool_info_loaded: bool,
}

#[derive(PartialEq)]
enum Tab {
    LiquidityWalls,
    DbExplorer,
    PoolInfo,
    Settings,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::LiquidityWalls
    }
}

impl TelOnChainUI {
    pub fn new(_cc: &CreationContext) -> Self {
        let mut app = TelOnChainUI {
            api_status: "Connecting...".to_string(),
            selected_dex: "uniswap_v3".to_string(),
            available_dexes: vec![
                "uniswap_v2".to_string(),
                "uniswap_v3".to_string(),
                "sushiswap".to_string(),
            ],
            selected_chain_id: 1,
            available_chain_ids: vec![1, 137, 42161, 10],
            token0_address: "".to_string(),
            token1_address: "".to_string(),
            available_tokens: HashMap::new(),
            liquidity_data: None,
            liquidity_promise: None,
            db_path: DEFAULT_DB_PATH.to_string(),
            db_pools: Vec::new(),
            db_tokens: Vec::new(),
            db_distributions: Vec::new(),
            db_query_status: "Not connected".to_string(),
            selected_tab: Tab::default(),
            selected_pool_idx: None,
            pool_info_loaded: false,
        };

        app.available_tokens.insert(
            1,
            vec!["ETH".to_string(), "USDC".to_string(), "WBTC".to_string()],
        );
        app.available_tokens.insert(
            137,
            vec!["MATIC".to_string(), "USDC".to_string(), "WETH".to_string()],
        );

        app.check_api_connection();

        app
    }

    fn check_api_connection(&mut self) {
        let client = reqwest::Client::new();
        let request = client.get(format!("{}/health", API_BASE_URL)).build().ok();

        if let Some(req) = request {
            let fut = async move {
                match client.execute(req).await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            Ok("Connected".to_string())
                        } else {
                            Err(format!("API error: {}", resp.status()))
                        }
                    }
                    Err(e) => Err(format!("Connection error: {}", e)),
                }
            };

            let _promise = Promise::spawn_thread("api_check", move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(fut)
            });
        } else {
            self.api_status = "Failed to build request".to_string();
        }
    }

    fn fetch_liquidity_walls(&mut self, ctx: &egui::Context) {
        if self.token0_address.is_empty() || self.token1_address.is_empty() {
            self.api_status = "Please enter token addresses".to_string();
            return;
        }

        self.api_status = "Fetching liquidity walls...".to_string();
        let client = reqwest::Client::new();
        let token0 = self.token0_address.clone();
        let token1 = self.token1_address.clone();
        let dex = self.selected_dex.clone();
        let chain_id = self.selected_chain_id;

        let url = format!(
            "{}/v1/liquidity/walls/{}/{}?dex={}&chain_id={}",
            API_BASE_URL, token0, token1, dex, chain_id
        );

        let fut = async move {
            let res = client.get(url).send().await;
            match res {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<LiquidityWallsResponse>().await {
                            Ok(data) => Ok(data),
                            Err(e) => Err(format!("Failed to parse response: {}", e)),
                        }
                    } else {
                        Err(format!("API error: {}", response.status()))
                    }
                }
                Err(e) => Err(format!("Request error: {}", e)),
            }
        };

        let ctx_clone = ctx.clone();
        self.liquidity_promise = Some(Promise::spawn_thread("fetch_liquidity", move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(fut);
            ctx_clone.request_repaint();
            result
        }));
    }

    fn query_database(&mut self) {
        let db_path_str = self.db_path.clone();
        let path = Path::new(&db_path_str);
        let conn = match Connection::open(path) {
            Ok(conn) => conn,
            Err(e) => {
                self.db_query_status = format!("Failed to open database: {}", e);
                return;
            }
        };

        // Initialize schema if tables don't exist
        let init_res = (|| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS tokens (
                    address TEXT PRIMARY KEY,
                    chain_id INTEGER NOT NULL,
                    name TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    decimals INTEGER NOT NULL
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS pools (
                    address TEXT PRIMARY KEY,
                    chain_id INTEGER NOT NULL,
                    dex TEXT NOT NULL,
                    token0_address TEXT NOT NULL,
                    token1_address TEXT NOT NULL,
                    fee INTEGER,
                    FOREIGN KEY (token0_address) REFERENCES tokens (address),
                    FOREIGN KEY (token1_address) REFERENCES tokens (address)
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS liquidity_distributions (
                    token0_address TEXT NOT NULL,
                    token1_address TEXT NOT NULL,
                    dex TEXT NOT NULL,
                    chain_id INTEGER NOT NULL,
                    data TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    PRIMARY KEY (token0_address, token1_address, dex, chain_id),
                    FOREIGN KEY (token0_address) REFERENCES tokens (address),
                    FOREIGN KEY (token1_address) REFERENCES tokens (address)
                )",
                [],
            )?;
            Ok::<(), rusqlite::Error>(())
        })();

        if let Err(e) = init_res {
            self.db_query_status = format!("Failed to initialize schema: {}", e);
            return;
        }

        // Now query the data
        self.query_pools(&conn);
        self.query_tokens(&conn);
        self.query_distributions(&conn);
        self.db_query_status = format!(
            "DB queries completed: {} pools, {} tokens, {} distributions",
            self.db_pools.len(),
            self.db_tokens.len(),
            self.db_distributions.len()
        );
    }

    fn query_pools(&mut self, conn: &Connection) {
        self.db_pools.clear();
        let sql = "SELECT address, dex, chain_id, token0_address, token1_address, fee FROM pools LIMIT 100";
        match conn.prepare(sql) {
            Ok(mut stmt) => {
                let pool_iter = stmt.query_map([], |row| {
                    Ok(DbPool {
                        address: row.get(0)?,
                        dex: row.get(1)?,
                        chain_id: row.get(2)?,
                        token0: row.get(3)?,
                        token1: row.get(4)?,
                        fee: row.get(5)?,
                    })
                });
                if let Ok(pools) = pool_iter {
                    for pool in pools {
                        if let Ok(pool) = pool {
                            self.db_pools.push(pool);
                        }
                    }
                }
            }
            Err(e) => {
                self.db_query_status = format!("Failed to prepare pool query: {}", e);
            }
        }
    }

    fn query_tokens(&mut self, conn: &Connection) {
        self.db_tokens.clear();
        let sql = "SELECT address, name, symbol, decimals, chain_id FROM tokens LIMIT 100";
        match conn.prepare(sql) {
            Ok(mut stmt) => {
                let token_iter = stmt.query_map([], |row| {
                    Ok(DbToken {
                        address: row.get(0)?,
                        name: row.get(1)?,
                        symbol: row.get(2)?,
                        decimals: row.get(3)?,
                        chain_id: row.get(4)?,
                    })
                });
                if let Ok(tokens) = token_iter {
                    for token in tokens {
                        if let Ok(token) = token {
                            self.db_tokens.push(token);
                        }
                    }
                }
            }
            Err(e) => {
                self.db_query_status = format!("Failed to prepare token query: {}", e);
            }
        }
    }

    fn query_distributions(&mut self, conn: &Connection) {
        self.db_distributions.clear();
        let sql = "SELECT pool_address, token0_address, token1_address, timestamp, data FROM liquidity_distributions LIMIT 100";
        match conn.prepare(sql) {
            Ok(mut stmt) => {
                let dist_iter = stmt.query_map([], |row| {
                    let dist_json: String = row.get(4)?;
                    let price_points = match serde_json::from_str::<serde_json::Value>(&dist_json) {
                        Ok(json) => json
                            .as_object()
                            .and_then(|obj| obj.get("price_levels"))
                            .and_then(|levels| levels.as_array())
                            .map(|arr| arr.len())
                            .unwrap_or(0),
                        Err(_) => 0,
                    };

                    Ok(DbLiquidityDistribution {
                        pool_address: row.get(0)?,
                        token0_address: row.get(1)?,
                        token1_address: row.get(2)?,
                        timestamp: row.get(3)?,
                        price_points,
                    })
                });
                if let Ok(distributions) = dist_iter {
                    for dist in distributions {
                        if let Ok(dist) = dist {
                            self.db_distributions.push(dist);
                        }
                    }
                }
            }
            Err(e) => {
                self.db_query_status = format!("Failed to prepare distribution query: {}", e);
            }
        }
    }

    fn load_pool_info(&mut self) {
        use rusqlite::{params, Connection};

        self.db_pools.clear();
        let db_path_str = self.db_path.clone();
        let path = Path::new(&db_path_str);
        if !path.exists() {
            self.db_query_status = format!("DB not found: {}", self.db_path);
            return;
        }

        let conn = match Connection::open(path) {
            Ok(c) => c,
            Err(e) => {
                self.db_query_status = e.to_string();
                return;
            }
        };

        let sql = "SELECT address, dex, chain_id, token0_address, token1_address, fee
                   FROM pools WHERE dex = ?1 AND chain_id = ?2 LIMIT 200";

        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                self.db_query_status = e.to_string();
                return;
            }
        };

        let rows = stmt.query_map(params![&self.selected_dex, self.selected_chain_id], |r| {
            Ok(DbPool {
                address: r.get(0)?,
                dex: r.get(1)?,
                chain_id: r.get(2)?,
                token0: r.get(3)?,
                token1: r.get(4)?,
                fee: r.get(5)?,
            })
        });

        match rows {
            Ok(iter) => {
                for p in iter.flatten() {
                    self.db_pools.push(p);
                }
                self.pool_info_loaded = true;
                self.db_query_status = format!("Loaded {} pools", self.db_pools.len());
            }
            Err(e) => self.db_query_status = e.to_string(),
        }
    }

    fn ui_liquidity_walls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("Liquidity Walls Visualization");

        ui.horizontal(|ui| {
            ui.label("DEX:");
            ComboBox::from_id_source("dex_select")
                .selected_text(&self.selected_dex)
                .show_ui(ui, |ui| {
                    for dex in &self.available_dexes {
                        ui.selectable_value(&mut self.selected_dex, dex.clone(), dex);
                    }
                });

            ui.label("Token0 Address:");
            ui.text_edit_singleline(&mut self.token0_address);
            ui.label("Token1 Address:");
            ui.text_edit_singleline(&mut self.token1_address);

            if ui.button("Fetch").clicked() {
                self.fetch_liquidity_walls(ctx);
            }
        });

        ui.separator();

        if let Some(data) = &self.liquidity_data {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Buy Walls");
                    self.show_walls(ui, &data.buy_walls, true);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.heading("Sell Walls");
                    self.show_walls(ui, &data.sell_walls, false);
                });
            });
        } else {
            ui.label("No data loaded. Fetch liquidity walls to see visualization.");
        }
    }

    fn ui_db_explorer(&mut self, ui: &mut Ui) {
        ui.heading("Database Explorer");

        ui.horizontal(|ui| {
            ui.label("Database Path:");
            ui.text_edit_singleline(&mut self.db_path);
            if ui.button("Query Database").clicked() {
                self.query_database();
            }
        });
        ui.label(&self.db_query_status);

        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Pools");
            Grid::new("pools_grid").striped(true).show(ui, |ui| {
                ui.label("Address");
                ui.label("DEX");
                ui.label("Chain ID");
                ui.label("Token0");
                ui.label("Token1");
                ui.label("Fee (0.0001% units)");
                ui.end_row();

                for pool in &self.db_pools {
                    ui.label(format!(
                        "{}...{}",
                        &pool.address[..6],
                        &pool.address[pool.address.len() - 4..]
                    ));
                    ui.label(&pool.dex);
                    ui.label(pool.chain_id.to_string());
                    ui.label(format!(
                        "{}...{}",
                        &pool.token0[..6],
                        &pool.token0[pool.token0.len() - 4..]
                    ));
                    ui.label(format!(
                        "{}...{}",
                        &pool.token1[..6],
                        &pool.token1[pool.token1.len() - 4..]
                    ));
                    ui.label(format!("{}", pool.fee));
                    ui.end_row();
                }
            });
        });
    }

    fn show_walls(&self, ui: &mut Ui, walls: &[LiquidityWall], is_buy: bool) {
        let color = if is_buy {
            Color32::DARK_GREEN
        } else {
            Color32::DARK_RED
        };

        for (i, wall) in walls.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Wall #{}", i + 1))
                        .color(color)
                        .strong(),
                );
                ui.label(format!(
                    "Price: {:.4} - {:.4}",
                    wall.price_lower, wall.price_upper
                ));
                ui.label(format!("Liquidity: ${:.2}", wall.liquidity_value));
            });

            if !wall.dex_sources.is_empty() {
                Grid::new(format!("dex_sources_{}", i)).show(ui, |ui| {
                    ui.label("DEX");
                    ui.label("Liquidity");
                    ui.end_row();
                    for (dex, amount) in &wall.dex_sources {
                        ui.label(dex);
                        ui.label(format!("${:.2}", amount));
                        ui.end_row();
                    }
                });
            }
            ui.separator();
        }

        if walls.is_empty() {
            ui.label("No walls detected");
        }
    }

    fn ui_pool_info(&mut self, ui: &mut Ui) {
        ui.heading("Pool Information");

        ui.horizontal(|ui| {
            ui.label("DEX:");
            ComboBox::from_id_source("pi_dex")
                .selected_text(&self.selected_dex)
                .show_ui(ui, |ui| {
                    for dex in &self.available_dexes {
                        ui.selectable_value(&mut self.selected_dex, dex.clone(), dex);
                    }
                });

            ui.label("Chain:");
            ComboBox::from_id_source("pi_chain")
                .selected_text(self.selected_chain_id.to_string())
                .show_ui(ui, |ui| {
                    for id in &self.available_chain_ids {
                        ui.selectable_value(&mut self.selected_chain_id, *id, id.to_string());
                    }
                });

            if ui.button("Load Pools").clicked() {
                self.pool_info_loaded = false;
                self.selected_pool_idx = None;
            }
        });

        if !self.pool_info_loaded {
            self.load_pool_info();
        }

        ui.label(RichText::new(&self.db_query_status).color(Color32::GOLD));
        ui.separator();

        if self.db_pools.is_empty() {
            ui.label("No pools found for current filter.");
            return;
        }

        ui.horizontal(|ui| {
            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for (idx, p) in self.db_pools.iter().enumerate() {
                    let short = format!(
                        "{}...{}",
                        &p.address[..6],
                        &p.address[p.address.len() - 4..]
                    );
                    if ui
                        .selectable_label(self.selected_pool_idx == Some(idx), short)
                        .clicked()
                    {
                        self.selected_pool_idx = Some(idx);
                    }
                }
            });

            ui.separator();

            if let Some(i) = self.selected_pool_idx {
                let p = &self.db_pools[i];
                ui.vertical(|ui| {
                    ui.heading("Selected Pool");
                    ui.label(format!("Address  : {}", p.address));
                    ui.label(format!("DEX      : {}", p.dex));
                    ui.label(format!("Chain ID : {}", p.chain_id));
                    ui.label(format!("Token 0  : {}", p.token0));
                    ui.label(format!("Token 1  : {}", p.token1));
                    ui.label(format!("Fee      : {} (x 0.0001%)", p.fee));
                });
            } else {
                ui.label("Select a pool to see details.");
            }
        });
    }

    fn ui_settings(&mut self, ui: &mut Ui) {
        ui.heading("Settings");

        ui.horizontal(|ui| {
            ui.label("API URL:");
            ui.label(API_BASE_URL);
        });

        if ui.button("Check API Connection").clicked() {
            self.check_api_connection();
        }

        ui.separator();
        ui.label("API Status: ");
        ui.label(
            RichText::new(&self.api_status).color(if self.api_status == "Connected" {
                Color32::GREEN
            } else {
                Color32::RED
            }),
        );
    }
}

impl App for TelOnChainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.liquidity_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(data) => {
                        self.api_status = "Data loaded successfully".to_string();
                        self.liquidity_data = Some(Arc::new(data.clone()));
                    }
                    Err(e) => {
                        self.api_status = format!("Error: {}", e);
                    }
                }
                self.liquidity_promise = None;
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Tel-On-Chain Debug UI");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let status_text = if self.api_status == "Connected" {
                        RichText::new("● Connected").color(Color32::GREEN)
                    } else {
                        RichText::new(format!("● {}", self.api_status)).color(Color32::RED)
                    };
                    ui.label(status_text);
                    if ui.button("Refresh").clicked() {
                        self.check_api_connection();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::LiquidityWalls,
                    "Liquidity Walls",
                );
                ui.selectable_value(&mut self.selected_tab, Tab::DbExplorer, "DB Explorer");
                ui.selectable_value(&mut self.selected_tab, Tab::PoolInfo, "Pool Info");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::LiquidityWalls => self.ui_liquidity_walls(ui, ctx),
            Tab::DbExplorer => self.ui_db_explorer(ui),
            Tab::PoolInfo => self.ui_pool_info(ui),
            Tab::Settings => self.ui_settings(ui),
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("API Status: {}", self.api_status));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Tel-On-Chain API Debug Tool");
                });
            });
        });
    }
}
