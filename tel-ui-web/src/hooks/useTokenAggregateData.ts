import { useState, useEffect, useCallback } from 'react';
import { apiClient } from '@/lib/api';
import { LiquidityDistribution } from '@/types/api';

interface UseTokenAggregateDataParams {
  tokenAddress?: string;
  dex?: string;
  chainId?: number;
}

interface UseTokenAggregateDataResult {
  data: LiquidityDistribution | null;
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useTokenAggregateData({
  tokenAddress,
  dex = 'all',
  chainId = 1,
}: UseTokenAggregateDataParams): UseTokenAggregateDataResult {
  const [data, setData] = useState<LiquidityDistribution | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    if (!tokenAddress) {
      setData(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const result = await apiClient.getTokenAggregateLiquidity(
        tokenAddress,
        dex,
        chainId
      );
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch aggregate liquidity data');
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [tokenAddress, dex, chainId]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const refetch = useCallback(() => {
    fetchData();
  }, [fetchData]);

  return {
    data,
    loading,
    error,
    refetch,
  };
}