import { useMemo } from 'react';
import useSWR from 'swr';
import { apiClient } from '@/lib/api';
import { LiquidityWallsQuery } from '@/types/api';
import { formatPriceRange } from '@/lib/utils';

export function useLiquidityData(
  token0Address: string | null,
  token1Address: string | null,
  params?: LiquidityWallsQuery,
  priceType: 'wall' | 'current' = 'wall',
  refreshInterval?: number
) {
  const key = useMemo(() => {
    if (!token0Address || !token1Address) return null;
    return ['liquidity-walls', token0Address, token1Address, params];
  }, [token0Address, token1Address, params]);

  const { data, error, isLoading, mutate } = useSWR(
    key,
    async ([, token0, token1, queryParams]: [string, string, string, LiquidityWallsQuery?]) => {
      return apiClient.getLiquidityWalls(token0, token1, queryParams);
    },
    {
      refreshInterval: refreshInterval || 30000, // 30 seconds default
      revalidateOnFocus: true,
      revalidateOnReconnect: true,
    }
  );

  const processedData = useMemo(() => {
    if (!data) return null;

    // Select sell walls based on price type
    const sellWalls = priceType === 'wall' ? data.sell_walls_in_wall_price : data.sell_walls_in_current_price;

    // Calculate total liquidity values
    const totalBuyLiquidity = data.buy_walls.reduce(
      (sum, wall) => sum + wall.liquidity_value,
      0
    );
    const totalSellLiquidity = sellWalls.reduce(
      (sum, wall) => sum + wall.liquidity_value,
      0
    );

    // Find strongest walls
    const strongestBuyWall = data.buy_walls.reduce(
      (strongest, wall) =>
        wall.liquidity_value > strongest.liquidity_value ? wall : strongest,
      data.buy_walls[0]
    );
    const strongestSellWall = sellWalls.reduce(
      (strongest, wall) =>
        wall.liquidity_value > strongest.liquidity_value ? wall : strongest,
      sellWalls[0]
    );

    // Prepare chart data
    const chartData = [
      ...data.buy_walls.map((wall) => ({
        priceRange: formatPriceRange(wall.price_lower, wall.price_upper, data.token1.symbol),
        price: (wall.price_lower + wall.price_upper) / 2,
        buyLiquidity: wall.liquidity_value,
        sellLiquidity: 0,
        type: 'buy' as const,
        dexSources: wall.dex_sources,
      })),
      ...sellWalls.map((wall) => ({
        priceRange: formatPriceRange(wall.price_lower, wall.price_upper, data.token1.symbol),
        price: (wall.price_lower + wall.price_upper) / 2,
        buyLiquidity: 0,
        sellLiquidity: wall.liquidity_value,
        type: 'sell' as const,
        dexSources: wall.dex_sources,
      })),
    ].sort((a, b) => a.price - b.price);

    return {
      ...data,
      totalBuyLiquidity,
      totalSellLiquidity,
      strongestBuyWall,
      strongestSellWall,
      chartData,
    };
  }, [data, priceType]);

  return {
    data: processedData,
    error,
    isLoading,
    refresh: mutate,
  };
} 