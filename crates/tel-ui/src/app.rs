use eframe::{App, CreationContext};
use egui::{Color32, ComboBox, Grid, RichText, ScrollArea, Ui};
use egui_plot::{Bar, BarChart, Plot};
use poll_promise::Promise;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

// For direct database access
use rusqlite::Connection;
use std::path::Path;

// API endpoints
const API_BASE_URL: &str = "http://127.0.0.1:8081";
const DEFAULT_DB_PATH: &str = "sqlite_tel_on_chain.db";

// Type aliases from the main project to use with the API
type Address = alloy_primitives::Address;

use tel_core::models::LiquidityDistribution;

#[derive(Debug, Clone, Deserialize)]
struct Token {
    address: Address,
    symbol: String,
    name: String,
    decimals: u8,
    chain_id: u64,
}

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
struct DbPool {
    address: String,
    dex: String,
    chain_id: u64,
    token0: String,
    token1: String,
    fee: u64, // 0.0001%의 몇 배인지
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
    token0_address: String,
    token1_address: String,
    timestamp: i64,
    price_points: usize,
    distribution: Option<LiquidityDistribution>, // JSON 전체
}

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

    // Pool-Info tab state
    pool_info_loaded: bool,           // 첫 로드 여부
    selected_pool_idx: Option<usize>, // 선택된 풀 인덱스

    // UI tabs
    selected_tab: Tab,

    // DB Explorer tab state
    db_explorer_tab: DbExplorerTab,
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

#[derive(PartialEq)]
enum DbExplorerTab {
    Pools,
    Tokens,
    Distributions,
}

impl Default for DbExplorerTab {
    fn default() -> Self {
        DbExplorerTab::Pools
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
            selected_chain_id: 1,                         // Default to Ethereum
            available_chain_ids: vec![1, 137, 42161, 10], // Ethereum, Polygon, Arbitrum, Optimism
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
            pool_info_loaded: false,
            selected_pool_idx: None,
            selected_tab: Tab::default(),
            db_explorer_tab: DbExplorerTab::default(),
        };

        // Initialize with some dummy tokens for each chain
        app.available_tokens.insert(
            1,
            vec!["ETH".to_string(), "USDC".to_string(), "WBTC".to_string()],
        );
        app.available_tokens.insert(
            137,
            vec!["MATIC".to_string(), "USDC".to_string(), "WETH".to_string()],
        );

        // Check API connection on startup
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

            let mut promise = Promise::spawn_thread("api_check", move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(fut)
            });

            let ctx = egui::Context::default();
            promise.ready_mut().map(|result| {
                match result {
                    Ok(status) => self.api_status = status.to_string(),
                    Err(err) => self.api_status = err.clone(),
                }
                ctx.request_repaint();
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
        // Check if database file exists
        let path = Path::new(&self.db_path);
        if !path.exists() {
            self.db_query_status = format!("Database file not found: {}", self.db_path);
            return;
        }

        match Connection::open(path) {
            Ok(conn) => {
                self.query_pools(&conn);
                self.query_tokens(&conn);
                self.query_distributions(&conn);
                self.db_query_status = format!(
                    "Database queries completed: found {} pools, {} tokens, {} distributions",
                    self.db_pools.len(),
                    self.db_tokens.len(),
                    self.db_distributions.len()
                );
            }
            Err(e) => {
                self.db_query_status = format!("Failed to connect to database: {}", e);
            }
        }
    }

    /// Queries up to 100 liquidity pools from the database and updates the internal pool list.
    ///
    /// If the query fails, updates the database query status with an error message.
    fn query_pools(&mut self, conn: &Connection) {
        self.db_pools.clear();
        let sql = "SELECT address, dex, chain_id, token0_address, token1_address, fee FROM pools LIMIT 100";
        match conn.prepare(sql) {
            Ok(mut stmt) => {
                match stmt.query_map([], |row| {
                    Ok(DbPool {
                        address: row.get(0)?,
                        dex: row.get(1)?,
                        chain_id: row.get(2)?,
                        token0: row.get(3)?,
                        token1: row.get(4)?,
                        fee: row.get(5)?,
                    })
                }) {
                    Ok(pools) => {
                        for pool in pools {
                            if let Ok(pool) = pool {
                                self.db_pools.push(pool);
                            }
                        }
                    }
                    Err(e) => {
                        self.db_query_status = format!("Failed to query pools: {}", e);
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
                match stmt.query_map([], |row| {
                    Ok(DbToken {
                        address: row.get(0)?,
                        name: row.get(1)?,
                        symbol: row.get(2)?,
                        decimals: row.get(3)?,
                        chain_id: row.get(4)?,
                    })
                }) {
                    Ok(tokens) => {
                        for token in tokens {
                            if let Ok(token) = token {
                                self.db_tokens.push(token);
                            }
                        }
                    }
                    Err(e) => {
                        self.db_query_status = format!("Failed to query tokens: {}", e);
                    }
                }
            }
            Err(e) => {
                self.db_query_status = format!("Failed to prepare token query: {}", e);
            }
        }
    }

    /// Queries up to 100 liquidity distribution records from the database and updates the application's state.
    ///
    /// For each distribution, parses the JSON field to count the number of price points and stores the result in `db_distributions`.
    /// Updates `db_query_status` with an error message if the query fails.
    fn query_distributions(&mut self, conn: &Connection) {
        self.db_distributions.clear();
        let sql = "SELECT token0_address, token1_address, dex, chain_id, data, timestamp FROM liquidity_distributions LIMIT 100";
        match conn.prepare(sql) {
            Ok(mut stmt) => {
                match stmt.query_map([], |row| {
                    let data: String = row.get(4)?;
                    let distribution: LiquidityDistribution = serde_json::from_str(&data)
                        .unwrap_or_else(|_| LiquidityDistribution {
                            token0: tel_core::models::Token {
                                address: alloy_primitives::Address::default(),
                                symbol: String::new(),
                                name: String::new(),
                                decimals: 0,
                                chain_id: 0,
                            },
                            token1: tel_core::models::Token {
                                address: alloy_primitives::Address::default(),
                                symbol: String::new(),
                                name: String::new(),
                                decimals: 0,
                                chain_id: 0,
                            },
                            dex: String::new(),
                            chain_id: 0,
                            price_levels: vec![],
                            timestamp: chrono::Utc::now(),
                        });
                    let price_points = distribution.price_levels.len();
                    Ok(DbLiquidityDistribution {
                        token0_address: row.get(0)?,
                        token1_address: row.get(1)?,
                        timestamp: row.get(5)?,
                        price_points,
                        distribution: Some(distribution),
                    })
                }) {
                    Ok(distributions) => {
                        for dist in distributions {
                            if let Ok(dist) = dist {
                                self.db_distributions.push(dist);
                            }
                        }
                    }
                    Err(e) => {
                        self.db_query_status = format!("Failed to query distributions: {}", e);
                    }
                }
            }
            Err(e) => {
                self.db_query_status = format!("Failed to prepare distribution query: {}", e);
            }
        }
    }

    /// Loads pool records from the database filtered by the selected DEX and chain ID.
    ///
    /// If the pools have already been loaded, the function returns immediately. Otherwise, it queries up to 200 pools matching the current DEX and chain selection, updates the internal pool list, and sets the query status message. If the database file does not exist or a query error occurs, the status message is updated accordingly.
    fn load_pool_info(&mut self) {
        // 이미 로드했다면 스킵 (새로고침 버튼으로 강제 갱신 가능)
        if self.pool_info_loaded {
            return;
        }

        // DB 경로 확인
        let path = std::path::Path::new(&self.db_path);
        if !path.exists() {
            self.db_query_status = format!("DB file not found: {}", self.db_path);
            return;
        }

        if let Ok(conn) = rusqlite::Connection::open(path) {
            self.db_pools.clear();

            let sql = "SELECT address, dex, chain_id, token0_address, token1_address \
                   FROM pools WHERE dex = ?1 AND chain_id = ?2 LIMIT 200";
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(e) => {
                    self.db_query_status = e.to_string();
                    return;
                }
            };

            let iter = stmt.query_map(
                rusqlite::params![self.selected_dex, self.selected_chain_id],
                |row| {
                    Ok(DbPool {
                        address: row.get(0)?,
                        dex: row.get(1)?,
                        chain_id: row.get(2)?,
                        token0: row.get(3)?,
                        token1: row.get(4)?,
                        fee: row.get(5)?,
                    })
                },
            );

            if let Ok(rows) = iter {
                for p in rows.flatten() {
                    self.db_pools.push(p);
                }
                self.pool_info_loaded = true;
                self.db_query_status = format!("Loaded {} pools", self.db_pools.len());
            }
        }
    }

    fn show_liquidity_distribution(&self, ui: &mut Ui, distribution: &DbLiquidityDistribution) {
        if let Some(dist) = &distribution.distribution {
            ui.heading("Liquidity Distribution");
            ui.horizontal(|ui| {
                ui.label("Token0 Address:");
                ui.label(format!("{}", dist.token0.address));
            });
            ui.horizontal(|ui| {
                ui.label("Token1 Address:");
                ui.label(format!("{}", dist.token1.address));
            });
            ui.horizontal(|ui| {
                ui.label("DEX:");
                ui.label(&dist.dex);
            });
            ui.horizontal(|ui| {
                ui.label("Chain ID:");
                ui.label(format!("{}", dist.chain_id));
            });
            ui.horizontal(|ui| {
                ui.label("Timestamp:");
                let ts = dist.timestamp.format("%Y-%m-%d %H:%M:%S");
                ui.label(format!("{}", ts));
            });
            ui.separator();
            ui.heading("Price Levels");
            for (i, level) in dist.price_levels.iter().enumerate() {
                ui.collapsing(format!("Price Level {}", i + 1), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Price:");
                        ui.label(format!("{:?}", level.price));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Token0 Liquidity:");
                        ui.label(format!("{:?}", level.token0_liquidity));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Token1 Liquidity:");
                        ui.label(format!("{:?}", level.token1_liquidity));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Timestamp:");
                        let ts = level.timestamp.format("%Y-%m-%d %H:%M:%S");
                        ui.label(format!("{}", ts));
                    });
                });
            }
        } else {
            ui.label("No distribution data");
        }
    }
}

impl App for TelOnChainUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if we received data from the API
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

impl TelOnChainUI {
    fn ui_liquidity_walls(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("Liquidity Walls Visualization");

        ui.horizontal(|ui| {
            ui.label("Chain:");
            ComboBox::from_id_source("chain_select")
                .selected_text(format!("{}", self.selected_chain_id))
                .show_ui(ui, |ui| {
                    for chain_id in &self.available_chain_ids {
                        let chain_name = match chain_id {
                            1 => "Ethereum",
                            137 => "Polygon",
                            42161 => "Arbitrum",
                            10 => "Optimism",
                            _ => "Unknown",
                        };
                        ui.selectable_value(
                            &mut self.selected_chain_id,
                            *chain_id,
                            format!("{} ({})", chain_name, chain_id),
                        );
                    }
                });

            ui.label("DEX:");
            ComboBox::from_id_source("dex_select")
                .selected_text(&self.selected_dex)
                .show_ui(ui, |ui| {
                    for dex in &self.available_dexes {
                        ui.selectable_value(&mut self.selected_dex, dex.clone(), dex);
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Token 0:");
            ui.text_edit_singleline(&mut self.token0_address);

            ui.label("Token 1:");
            ui.text_edit_singleline(&mut self.token1_address);

            if ui.button("Fetch Data").clicked() {
                self.fetch_liquidity_walls(ctx);
            }
        });

        ui.separator();

        if let Some(data) = &self.liquidity_data {
            ui.heading(format!(
                "{}/{} Price: {:.6}",
                data.token0.symbol, data.token1.symbol, data.price
            ));

            ui.horizontal(|ui| {
                // Buy walls (support)
                ui.vertical(|ui| {
                    ui.heading("Buy Walls (Support)");
                    ScrollArea::vertical().show(ui, |ui| {
                        self.show_walls(ui, &data.buy_walls, true);
                    });
                });

                ui.separator();

                // Sell walls (resistance)
                ui.vertical(|ui| {
                    ui.heading("Sell Walls (Resistance)");
                    ScrollArea::vertical().show(ui, |ui| {
                        self.show_walls(ui, &data.sell_walls, false);
                    });
                });
            });

            ui.separator();

            // Liquidity chart visualization
            Plot::new("liquidity_chart")
                .height(200.0)
                .show(ui, |plot_ui| {
                    // Buy walls
                    let buy_bars: Vec<Bar> = data
                        .buy_walls
                        .iter()
                        .map(|wall| {
                            let avg_price = (wall.price_lower + wall.price_upper) / 2.0;
                            Bar::new(avg_price, wall.liquidity_value)
                                .width(wall.price_upper - wall.price_lower)
                                .fill(Color32::from_rgb(0, 150, 0))
                        })
                        .collect();

                    // Sell walls
                    let sell_bars: Vec<Bar> = data
                        .sell_walls
                        .iter()
                        .map(|wall| {
                            let avg_price = (wall.price_lower + wall.price_upper) / 2.0;
                            Bar::new(avg_price, wall.liquidity_value)
                                .width(wall.price_upper - wall.price_lower)
                                .fill(Color32::from_rgb(150, 0, 0))
                        })
                        .collect();

                    plot_ui.bar_chart(BarChart::new(buy_bars).name("Buy Walls"));
                    plot_ui.bar_chart(BarChart::new(sell_bars).name("Sell Walls"));
                });
        } else {
            ui.label("No data available. Enter token addresses and fetch data.");
        }
    }

    /// Renders the Database Explorer tab, allowing users to input a database path, query the SQLite database, and view pool data.
    ///
    /// Displays the current query status, provides controls for querying, and shows a tabbed interface for pools, tokens, and distributions. Pool data is presented in a grid with truncated addresses for readability. If no data is available, prompts the user to query the database first.
    fn ui_db_explorer(&mut self, ui: &mut Ui) {
        ui.heading("Database Explorer");

        ui.horizontal(|ui| {
            ui.label("Database Path:");
            ui.text_edit_singleline(&mut self.db_path);

            // Query 버튼을 각 탭에 맞게 동작하도록 변경
            let query_label = match self.db_explorer_tab {
                DbExplorerTab::Pools => "Query Pools",
                DbExplorerTab::Tokens => "Query Tokens",
                DbExplorerTab::Distributions => "Query Distributions",
            };
            if ui.button(query_label).clicked() {
                let path = Path::new(&self.db_path);
                if !path.exists() {
                    self.db_query_status = format!("Database file not found: {}", self.db_path);
                    return;
                }
                match Connection::open(path) {
                    Ok(conn) => match self.db_explorer_tab {
                        DbExplorerTab::Pools => {
                            self.query_pools(&conn);
                            self.db_query_status = format!("Queried {} pools", self.db_pools.len());
                        }
                        DbExplorerTab::Tokens => {
                            self.query_tokens(&conn);
                            self.db_query_status =
                                format!("Queried {} tokens", self.db_tokens.len());
                        }
                        DbExplorerTab::Distributions => {
                            self.query_distributions(&conn);
                            self.db_query_status =
                                format!("Queried {} distributions", self.db_distributions.len());
                        }
                    },
                    Err(e) => {
                        self.db_query_status = format!("Failed to connect to database: {}", e);
                    }
                }
            }
        });

        ui.label(RichText::new(&self.db_query_status).color(
            if self.db_query_status.starts_with("Failed") {
                Color32::RED
            } else {
                Color32::GOLD
            },
        ));

        ui.separator();

        // Use tabs for different database tables
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.db_explorer_tab == DbExplorerTab::Pools,
                    format!("Pools ({})", self.db_pools.len()),
                )
                .clicked()
            {
                self.db_explorer_tab = DbExplorerTab::Pools;
            }
            if ui
                .selectable_label(
                    self.db_explorer_tab == DbExplorerTab::Tokens,
                    format!("Tokens ({})", self.db_tokens.len()),
                )
                .clicked()
            {
                self.db_explorer_tab = DbExplorerTab::Tokens;
            }
            if ui
                .selectable_label(
                    self.db_explorer_tab == DbExplorerTab::Distributions,
                    format!("Distributions ({})", self.db_distributions.len()),
                )
                .clicked()
            {
                self.db_explorer_tab = DbExplorerTab::Distributions;
            }
        });

        ui.separator();

        match self.db_explorer_tab {
            DbExplorerTab::Pools => {
                if !self.db_pools.is_empty() {
                    ui.heading("Pool Data");
                    Grid::new("pools_grid").striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Address").strong());
                        ui.label(RichText::new("DEX").strong());
                        ui.label(RichText::new("Chain").strong());
                        ui.label(RichText::new("Token 0").strong());
                        ui.label(RichText::new("Token 1").strong());
                        ui.label(RichText::new("Fee").strong());
                        ui.end_row();
                        for pool in &self.db_pools {
                            ui.label(&pool.address);
                            ui.label(&pool.dex);
                            ui.label(format!("{}", pool.chain_id));
                            ui.label(&pool.token0);
                            ui.label(&pool.token1);
                            ui.label(format!("{}", pool.fee));
                            ui.end_row();
                        }
                    });
                } else {
                    ui.label("No pool data available. Query the database first.");
                }
            }
            DbExplorerTab::Tokens => {
                if !self.db_tokens.is_empty() {
                    ui.heading("Token Data");
                    Grid::new("tokens_grid").striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Address").strong());
                        ui.label(RichText::new("Symbol").strong());
                        ui.label(RichText::new("Name").strong());
                        ui.label(RichText::new("Decimals").strong());
                        ui.label(RichText::new("Chain").strong());
                        ui.end_row();
                        for token in &self.db_tokens {
                            let short_address = format!(
                                "{}...{}",
                                &token.address[0..6],
                                &token.address[token.address.len() - 4..]
                            );
                            ui.label(short_address);
                            ui.label(&token.symbol);
                            ui.label(&token.name);
                            ui.label(format!("{}", token.decimals));
                            ui.label(format!("{}", token.chain_id));
                            ui.end_row();
                        }
                    });
                } else {
                    ui.label("No token data available. Query the database first.");
                }
            }
            DbExplorerTab::Distributions => {
                if !self.db_distributions.is_empty() {
                    ui.heading("Distribution Data");
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, dist) in self.db_distributions.iter().enumerate() {
                            ui.collapsing(format!("Distribution {}", i + 1), |ui| {
                                self.show_liquidity_distribution(ui, dist);
                            });
                            ui.separator();
                        }
                    });
                } else {
                    ui.label("No distribution data available. Query the database first.");
                }
            }
        }
    }

    /// Displays a list of liquidity walls with price ranges, liquidity values, and DEX breakdowns.
    ///
    /// Each wall is shown with its price range and total liquidity, color-coded by buy or sell side. If DEX source data is available, a breakdown of liquidity per DEX is displayed. If no walls are present, a message is shown.
    ///
    /// # Parameters
    /// - `walls`: Slice of liquidity walls to display.
    /// - `is_buy`: If true, walls are shown as buy walls (green); otherwise, as sell walls (red).
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `ui` is a mutable reference to an egui::Ui and `walls` is a Vec<LiquidityWall>.
    /// app.show_walls(ui, &walls, true); // Displays buy walls
    /// app.show_walls(ui, &walls, false); // Displays sell walls
    /// ```
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

            // Show DEX breakdown if available
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

    /// Renders the Pool Info tab, allowing users to filter, load, and browse pools by DEX and chain.
    ///
    /// Displays filter controls for DEX and chain selection, a button to load pools, and the current database query status.
    /// Shows a scrollable list of pools matching the selected filters. Selecting a pool displays its detailed information.
    /// If no pools are found, a message is shown instead.
    fn ui_pool_info(&mut self, ui: &mut Ui) {
        // 상단 필터
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
                .selected_text(format!("{}", self.selected_chain_id))
                .show_ui(ui, |ui| {
                    for id in &self.available_chain_ids {
                        ui.selectable_value(&mut self.selected_chain_id, *id, id.to_string());
                    }
                });

            if ui.button("Load Pools").clicked() {
                self.pool_info_loaded = false; // 강제 새로고침
                self.load_pool_info();
            }
        });

        // 처음 진입 시 자동 로드
        if !self.pool_info_loaded {
            self.load_pool_info();
        }

        ui.separator();
        ui.label(RichText::new(&self.db_query_status).color(Color32::GOLD));

        // 풀 리스트
        if self.db_pools.is_empty() {
            ui.label("No pools found for chosen filter.");
            return;
        }

        // 왼쪽: 리스트  |  오른쪽: 세부 정보
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
                    ui.heading("Pool Detail");
                    ui.label(format!("Address : {}", p.address));
                    ui.label(format!("DEX     : {}", p.dex));
                    ui.label(format!("Chain   : {}", p.chain_id));
                    ui.label(format!("Token 0 : {}", p.token0));
                    ui.label(format!("Token 1 : {}", p.token1));
                });
            } else {
                ui.label("Select a pool to see details.");
            }
        });
    }

    /// Renders the Settings tab UI, allowing users to view the API URL, check API connectivity, and see the current API connection status.
    ///
    /// # Examples
    ///
    /// ```
    /// // Within the egui update loop:
    /// tel_on_chain_ui.ui_settings(ui);
    /// ```
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

fn main() -> eframe::Result<()> {
    // Initialize logging for the UI
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Tel-On-Chain Debug UI",
        options,
        Box::new(|cc| Box::new(TelOnChainUI::new(cc))),
    )
}
