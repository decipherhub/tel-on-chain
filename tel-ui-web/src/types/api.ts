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
  sell_walls: LiquidityWall[];
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