# tel-on-chain Frontend

A modern Next.js frontend for visualizing on-chain liquidity and buy/sell walls across multiple DEXes.

![tel-on-chain dashboard](../assets/tel-on-chain.png)

## Features

- **Token Pair Analysis**: Select any token pair to analyze liquidity distribution
- **Multi-Chain Support**: Works with Ethereum, Polygon, Arbitrum, and Optimism
- **Multi-DEX Integration**: Aggregates data from Uniswap V2/V3, SushiSwap, Curve, and Balancer
- **Interactive Charts**: Visualize buy/sell walls with interactive bar charts
- **Real-time Data**: Auto-refreshing liquidity data with 30-second intervals
- **Support/Resistance Detection**: Identify key price levels with visual indicators
- **DEX Breakdown**: See liquidity sources from different DEXes
- **Responsive Design**: Works on desktop and mobile devices

## Tech Stack

- **Next.js 15**: React framework with App Router
- **TypeScript**: Type-safe development
- **Tailwind CSS**: Utility-first CSS framework
- **Recharts**: Data visualization library
- **SWR**: Data fetching and caching
- **React Hook Form**: Form management
- **Lucide React**: Modern icon library

## Getting Started

### Prerequisites

- Node.js 18+
- npm or yarn
- Running tel-api backend (see main project README)

### Installation

1. Clone the repository and navigate to the frontend:

   ```bash
   cd tel-ui-web
   ```

2. Install dependencies:

   ```bash
   npm install
   ```

3. Copy environment variables:

   ```bash
   cp .env.example .env.local
   ```

4. Update `.env.local` with your API URL if different from default:

   ```env
   NEXT_PUBLIC_API_BASE_URL=http://localhost:8081
   ```

5. Start the development server:

   ```bash
   npm run dev
   ```

6. Open [http://localhost:3000](http://localhost:3000) in your browser

## Usage

### Basic Flow

1. **Select Token Pair**: Enter token addresses or choose from popular tokens
2. **Choose Filters**: Select chain and optionally filter by specific DEX
3. **Analyze Liquidity**: Click "Analyze Liquidity" to fetch and visualize data
4. **Interpret Results**:
   - Green bars show buy liquidity (support levels)
   - Red bars show sell liquidity (resistance levels)
   - Current price is marked with a vertical line
   - Hover over bars for detailed information

### Understanding the Visualization

- **Buy Walls (Green)**: Represent liquidity available for buying, creating support levels
- **Sell Walls (Red)**: Represent liquidity available for selling, creating resistance levels
- **Price Ranges**: Each bar represents liquidity concentrated in a specific price range
- **DEX Sources**: Tooltip shows which DEXes contribute to each liquidity wall
- **Total Liquidity**: Summary statistics show overall market depth

## API Integration

The frontend communicates with the tel-api backend through these endpoints:

- `GET /v1/liquidity/walls/:token0/:token1` - Fetch liquidity walls data
- `GET /v1/tokens/:chain_id/:address` - Get token information
- `GET /health` - Health check

API responses are automatically cached and refreshed every 30 seconds using SWR.

## Development

### Project Structure

```
src/
├── app/                 # Next.js App Router pages
├── components/          # React components
│   ├── ui/             # Reusable UI components
│   ├── LiquidityChart.tsx
│   ├── StatsSummary.tsx
│   └── TokenSelector.tsx
├── hooks/              # Custom React hooks
├── lib/                # Utilities and API client
├── types/              # TypeScript type definitions
└── ...
```

### Key Components

- **TokenSelector**: Form for selecting token pairs and filters
- **LiquidityChart**: Interactive bar chart using Recharts
- **StatsSummary**: Key statistics and strongest walls display
- **useLiquidityData**: Custom hook for data fetching with SWR

### Environment Variables

- `NEXT_PUBLIC_API_BASE_URL`: Backend API base URL (required)

### Scripts

- `npm run dev`: Start development server
- `npm run build`: Build for production
- `npm run start`: Start production server
- `npm run lint`: Run ESLint

## Deployment

### Production Build

```bash
npm run build
npm run start
```

### Environment Setup

Ensure the following environment variables are set:

```env
NEXT_PUBLIC_API_BASE_URL=https://your-api-domain.com
NODE_ENV=production
```

### Docker (Optional)

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm install --production
COPY . .
RUN npm run build

EXPOSE 3000
CMD ["npm", "start"]
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see the [LICENSE](../LICENSE) file for details.

## Support

For issues and questions:

- Open an issue on [GitHub](https://github.com/decipherhub/tel-on-chain/issues)
- Check the main project [README](../README.md) for backend setup
