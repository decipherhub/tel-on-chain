import { ChainConfig, DexConfig } from '@/types/api';

export const SUPPORTED_CHAINS: ChainConfig[] = [
  {
    id: 1,
    name: 'Ethereum',
    rpcUrl: 'https://eth-mainnet.alchemyapi.io/v2/',
  },
  {
    id: 137,
    name: 'Polygon',
    rpcUrl: 'https://polygon-mainnet.alchemyapi.io/v2/',
  },
  {
    id: 42161,
    name: 'Arbitrum',
    rpcUrl: 'https://arb-mainnet.alchemyapi.io/v2/',
  },
  {
    id: 10,
    name: 'Optimism',
    rpcUrl: 'https://opt-mainnet.alchemyapi.io/v2/',
  },
];

export const SUPPORTED_DEXES: DexConfig[] = [
  {
    name: 'uniswap_v2',
    displayName: 'Uniswap V2',
    enabled: true,
  },
  {
    name: 'uniswap_v3',
    displayName: 'Uniswap V3',
    enabled: true,
  },
  {
    name: 'sushiswap',
    displayName: 'SushiSwap',
    enabled: true,
  },
  {
    name: 'curve',
    displayName: 'Curve',
    enabled: true,
  },
  {
    name: 'balancer',
    displayName: 'Balancer',
    enabled: true,
  },
];

export const POPULAR_TOKENS: Record<number, Array<{ address: string; symbol: string; name: string }>> = {
  1: [ // Ethereum
    {
      address: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      symbol: 'WETH',
      name: 'Wrapped Ether',
    },
    {
      address: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
      symbol: 'USDC',
      name: 'USD Coin',
    },
    {
      address: '0xdAC17F958D2ee523a2206206994597C13D831ec7',
      symbol: 'USDT',
      name: 'Tether USD',
    },
    {
      address: '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599',
      symbol: 'WBTC',
      name: 'Wrapped Bitcoin',
    },
  ],
}; 