# tel-on-chain: On-Chain Buy/Sell Wall Visualizer

![tel-on-chain header image](./assets/tel-on-chain.png)

## What is tel-on-chain?

`tel-on-chain` is an aggregator platform that collects and visualizes buy/sell wall data from on-chain DEXs (such as Uniswap, Curve, Balancer, etc.). It helps traders intuitively identify key support/resistance price levels and liquidity concentration zones.

The platform integrates pool data from ethereum and multiple protocols, making them accessible through a single interface.

Implementation formats:

- [API Server](docs/API_V1.md)
- Frontend dashboard

The name originates from the Hebrew word "tel", meaning "hill". It metaphorically represents an "on-chain hill", symbolizing the ebb and flow of liquidity on a chart.

## Why tel-on-chain?

### 1. Difficulty in Understanding On-Chain Liquidity

Current on-chain exchanges are AMM-based rather than orderbook-based, making it difficult to understand liquidity structures and lacking intuitive supply/demand level information.

### 2. Key Indicators for Market Timing and Risk Management

Buy/sell walls are meaningful indicators that can indirectly reveal the intentions of institutions and whales. There's a lack of tools that can show this information in real-time and in an integrated manner.

### 3. Enhanced Transparency

While CEX orderbooks can be manipulated by exchanges, on-chain supply/demand levels can be transparently verified, making them a reliable source of market information due to their inherent transparency.

By structuring and providing this data, we can democratize access to valuable trading insights, making them available to everyone in the DeFi ecosystem.

## How We're Building It

![diagram](./assets/diagram.png)

### 1. Data Collection (Indexing)

- Collect pool data from major DEXs (Uniswap v2/v3/v4, Curve, Balancer, etc.) via RPC using `alloy-rs`.
- Calculate price-based liquidity distribution (liquidity ticks)
- Advanced feature: Time machine - Visualize not only real-time but also historical liquidity data

### 2. Aggregation & Analysis

- Integrate liquidity maps by chain/DEX for specific asset pairs
- Analyze liquidity density by price range (e.g., buy walls concentrated between $2200-$2300)
- Provide supplementary indicators like swap impact calculations
- For any token address, aggregate and visualize buy/sell liquidity across all DEXes and pools containing that token, providing a comprehensive view of market depth
- Track liquidity provider positions with detailed breakdowns of their shares and percentage contributions to each pool

### 3. Visualization and Interface

- Heat map visualization of price-based supply/demand levels
- Automatic detection of support/resistance levels
- TradingView-style chart overlay functionality (to aid trading strategy development)
- Provide real-time data to clients via API
- Open-source release with a DeFiLlama-like approach

### 4. Future Expansion

- Implement an example client featuring price wall-based alerts using the tel-on-chain API for instance, sending notifications when a buy wall drops below a specified threshold.
- Comparative analysis between CEX orderbooks and on-chain supply/demand levels
- Track liquidity movement of specific address groups (e.g., whales, MEV bots)
- Integration of Liquidation Maps
  - Visualize the distribution of possible liquidation positions in on-chain money markets like Aave, Compound, Morpho
  - Example: If ETH drops below $1,950, $10M worth of liquidations could occur
  - Overlay buy walls and liquidation maps to show intuitively which price levels might trigger rapid price movements

## Getting Started

### Prerequisites

- Rust (stable or nightly)
- SQLite 3

### Installation

1. Clone the repository:

```bash
git clone https://github.com/decipherhub/tel-on-chain.git
cd tel-on-chain
```

2. Build the project:

```bash
cargo build --release
```

### Usage

Tel-on-chain provides two main utilities:

#### Running the API Server

```bash
cargo run --release -- api
```

This will start the API server on the configured host and port (default: `127.0.0.1:8080`).

#### Running the Indexer

To start the continuous indexer (collecting data from all configured DEXs):

```bash
cargo run --release -- index
```

To index a specific DEX and pool:

```bash
cargo run --release -- index --dex uniswap_v3 --pair 0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640
```

### Configuration

The configuration is stored in `config/default.toml`. You can specify:

- RPC endpoints for different blockchains
- Database connection settings
- API server settings
- DEXes to monitor and their factory addresses

### Project Structure

- `src/api`: API server implementation
- `src/config`: Configuration handling
- `src/core`: Core analysis logic (liquidity walls, support/resistance)
- `src/dexes`: DEX protocol adapters (Uniswap V2/V3, Sushiswap, etc.)
- `src/indexer`: Blockchain data indexing logic
- `src/models`: Data structures and types
- `src/providers`: Blockchain RPC providers
- `src/storage`: Database interaction
- `src/utils`: Utility functions

### API Documentation

API documentation is available at [docs/API_V1.md](docs/API_V1.md).

## Team

- [guzus](https://github.com/guzus)
  - CS major at Seoul National University
  - 3 years of experience in DeFi and MEV at a crypto trading firm
  - Winner at ETHGlobal Bangkok 2024 and Seoulana 2025 hackathons
- [0xGh-st](https://github.com/0xGh-st)
  - Dept. Of Financial Security, Blockchain major(master degree) at Korea University
  - Upside Academy(Upbit X Chainlight) 1st
  - Upbit D Conference(UDC) 2024 Speaker
  - Best Of the Best 12th Vulnerability Analysis
  - Ethcon Korea 2024 CTF Section Organizer
- [lee2020090791](https://github.com/lee2020090791)
  - CS major at Hanyang University
  - 1 year internship on OSDC labs
  - Blockchain core developer

## License

MIT
