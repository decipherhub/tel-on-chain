import { LiquidityWallsResponse, LiquidityWallsQuery, Token, Pool, PaginationParams } from '@/types/api';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:8081';

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;

    try {
      const response = await fetch(url, {
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
        },
        ...options,
      });

      if (!response.ok) {
        throw new Error(`API Error: ${response.status} ${response.statusText}`);
      }

      return await response.json();
    } catch (error) {
      console.error('API request failed:', error);
      throw error;
    }
  }

  async getLiquidityWalls(
    token0: string,
    token1: string,
    params?: LiquidityWallsQuery
  ): Promise<LiquidityWallsResponse> {
    const searchParams = new URLSearchParams();
    if (params?.dex) searchParams.append('dex', params.dex);
    if (params?.chain_id) searchParams.append('chain_id', params.chain_id.toString());

    const query = searchParams.toString() ? `?${searchParams.toString()}` : '';
    return this.request<LiquidityWallsResponse>(`/v1/liquidity/walls/${token0}/${token1}${query}`);
  }

  async getTokenInfo(chainId: number, address: string): Promise<Token> {
    return this.request<Token>(`/v1/tokens/${chainId}/${address}`);
  }

  async getPoolsByDex(dex: string, chainId: number, pagination?: PaginationParams): Promise<Pool[]> {
    const searchParams = new URLSearchParams();
    if (pagination?.page) searchParams.append('page', pagination.page.toString());
    if (pagination?.limit) searchParams.append('limit', pagination.limit.toString());

    const query = searchParams.toString() ? `?${searchParams.toString()}` : '';
    return this.request<Pool[]>(`/v1/pools/${dex}/${chainId}${query}`);
  }

  async getAllPools(chainId: number, pagination?: PaginationParams): Promise<Pool[]> {
    const searchParams = new URLSearchParams();
    if (pagination?.page) searchParams.append('page', pagination.page.toString());
    if (pagination?.limit) searchParams.append('limit', pagination.limit.toString());

    const query = searchParams.toString() ? `?${searchParams.toString()}` : '';
    return this.request<Pool[]>(`/v1/chains/${chainId}/pools${query}`);
  }

  async healthCheck(): Promise<void> {
    return this.request<void>('/health');
  }
}

export const apiClient = new ApiClient(); 