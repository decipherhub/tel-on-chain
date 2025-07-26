export interface Token {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  chain_id: number;
}

export interface LiquidityWall {
  price_lower: number;
  price_upper: number;
  liquidity_value: number;
  dex_sources: Record<string, number>;
}

export interface LiquidityWallsResponse {
  token0: Token;
  token1: Token;
  price: number;
  buy_walls: LiquidityWall[];
  sell_walls_in_wall_price: LiquidityWall[];
  sell_walls_in_current_price: LiquidityWall[];
  timestamp: string;
}

export interface ChainConfig {
  id: number;
  name: string;
  rpcUrl: string;
}

export interface DexConfig {
  name: string;
  displayName: string;
  enabled: boolean;
}

export interface LiquidityWallsQuery {
  dex?: string;
  chain_id?: number;
}

export interface PaginationParams {
  page?: number;
  limit?: number;
}

export interface ApiError {
  message: string;
  code: number;
}

export interface Pool {
  address: string;
  dex: string;
  chain_id: number;
  tokens: Token[];
  creation_block: number;
  creation_timestamp: string;
  last_updated_block: number;
  last_updated_timestamp: string;
  fee: number;
}

export interface PriceLiquidity {
  side: 'Buy' | 'Sell';
  lower_price: number;
  upper_price: number;
  token0_liquidity: number;
  token1_liquidity: number;
  timestamp: string;
}

export interface LiquidityDistribution {
  token0: Token;
  token1: Token;
  current_price: number;
  dex: string;
  chain_id: number;
  price_levels: PriceLiquidity[];
  timestamp: string;
} 