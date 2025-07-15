'use client';

import { useState, useEffect } from 'react';
import { Pool } from '@/types/api';
import { apiClient } from '@/lib/api';

interface UsePoolDataParams {
  chainId: number;
  dex?: string;
  autoRefresh?: boolean;
}

interface UsePoolDataResult {
  pools: Pool[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function usePoolData({ 
  chainId, 
  dex, 
  autoRefresh = false 
}: UsePoolDataParams): UsePoolDataResult {
  const [pools, setPools] = useState<Pool[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPools = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const fetchedPools = dex 
        ? await apiClient.getPoolsByDex(dex, chainId)
        : await apiClient.getAllPools(chainId);
      
      setPools(fetchedPools);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch pools');
    } finally {
      setLoading(false);
    }
  };

  const refresh = () => {
    fetchPools();
  };

  useEffect(() => {
    fetchPools();
  }, [chainId, dex]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(fetchPools, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, [autoRefresh, chainId, dex]);

  return {
    pools,
    loading,
    error,
    refresh,
  };
}