'use client';

import { useState, useEffect, useCallback } from 'react';
import { Pool, PaginationParams } from '@/types/api';
import { apiClient } from '@/lib/api';

interface UsePoolDataParams {
  chainId: number;
  dex?: string;
  autoRefresh?: boolean;
  page?: number;
  limit?: number;
}

interface UsePoolDataResult {
  pools: Pool[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
  totalPools: number;
}

export function usePoolData({ 
  chainId, 
  dex, 
  autoRefresh = false,
  page,
  limit
}: UsePoolDataParams): UsePoolDataResult {
  const [pools, setPools] = useState<Pool[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [totalPools, setTotalPools] = useState(0);

  const fetchPools = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const pagination = (page || limit) ? { page, limit } : undefined;
      
      const fetchedPools = dex 
        ? await apiClient.getPoolsByDex(dex, chainId, pagination)
        : await apiClient.getAllPools(chainId, pagination);
      
      setPools(fetchedPools);
      setTotalPools(fetchedPools.length);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch pools');
    } finally {
      setLoading(false);
    }
  }, [chainId, dex, page, limit]);

  const refresh = () => {
    fetchPools();
  };

  useEffect(() => {
    fetchPools();
  }, [fetchPools]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(fetchPools, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, [autoRefresh, fetchPools]);

  return {
    pools,
    loading,
    error,
    refresh,
    totalPools,
  };
}