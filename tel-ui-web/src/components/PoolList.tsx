'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { Pool } from '@/types/api';
import { apiClient } from '@/lib/api';
import { Loader2, Search, ExternalLink, Filter } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Pagination } from '@/components/ui/Pagination';

interface PoolListProps {
  onPoolSelect: (pool: Pool) => void;
  selectedPool?: Pool;
}

export function PoolList({ onPoolSelect, selectedPool }: PoolListProps) {
  const [pools, setPools] = useState<Pool[]>([]);
  const [filteredPools, setFilteredPools] = useState<Pool[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedDex, setSelectedDex] = useState<string>('all');
  const [chainId] = useState(1); // Default to Ethereum mainnet
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 100;

  const fetchPools = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const fetchedPools = await apiClient.getAllPools(chainId);
      setPools(fetchedPools);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch pools');
    } finally {
      setLoading(false);
    }
  }, [chainId]);

  const filterPools = useCallback(() => {
    let filtered = pools;

    // Filter by search term
    if (searchTerm) {
      filtered = filtered.filter(pool => 
        pool.tokens.some(token => 
          token.symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
          token.name.toLowerCase().includes(searchTerm.toLowerCase())
        ) || 
        pool.address.toLowerCase().includes(searchTerm.toLowerCase())
      );
    }

    // Filter by DEX
    if (selectedDex !== 'all') {
      filtered = filtered.filter(pool => pool.dex === selectedDex);
    }

    setFilteredPools(filtered);
  }, [pools, searchTerm, selectedDex]);

  useEffect(() => {
    fetchPools();
  }, [chainId, fetchPools]);

  useEffect(() => {
    filterPools();
    setCurrentPage(1); // Reset to first page when filters change
  }, [pools, searchTerm, selectedDex, filterPools]);

  const formatTokenPair = (pool: Pool) => {
    if (pool.tokens.length >= 2) {
      return `${pool.tokens[0].symbol}/${pool.tokens[1].symbol}`;
    }
    return pool.tokens[0]?.symbol || 'Unknown';
  };

  const formatFee = (fee: number) => {
    return (fee / 10000).toFixed(2) + '%';
  };

  const formatAddress = (address: string) => {
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  const getDexDisplayName = (dex: string) => {
    switch (dex) {
      case 'uniswap_v2':
        return 'Uniswap V2';
      case 'uniswap_v3':
        return 'Uniswap V3';
      case 'sushiswap':
        return 'SushiSwap';
      default:
        return dex;
    }
  };

  const getUniqueDeXes = () => {
    const dexes = [...new Set(pools.map(pool => pool.dex))];
    return dexes.sort();
  };

  const getPaginatedPools = () => {
    const startIndex = (currentPage - 1) * itemsPerPage;
    const endIndex = startIndex + itemsPerPage;
    return filteredPools.slice(startIndex, endIndex);
  };

  const totalPages = Math.ceil(filteredPools.length / itemsPerPage);

  const handlePageChange = (page: number) => {
    setCurrentPage(page);
  };

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin text-blue-600 mr-3" />
          <span className="text-gray-600">Loading pools...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="text-center py-8">
          <div className="text-red-600 mb-4">Error loading pools</div>
          <div className="text-gray-600 mb-4">{error}</div>
          <Button onClick={fetchPools} variant="outline" size="sm">
            Try Again
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow">
      <div className="p-6 border-b border-gray-200">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Available Pools</h2>
          <div className="text-sm text-gray-500">
            {filteredPools.length} of {pools.length} pools
            {filteredPools.length > itemsPerPage && (
              <span className="ml-2">
                (showing {getPaginatedPools().length} on page {currentPage} of {totalPages})
              </span>
            )}
          </div>
        </div>

        {/* Search and Filter */}
        <div className="flex flex-col sm:flex-row gap-4 mb-4">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
            <input
              type="text"
              placeholder="Search by token symbol, name, or address..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          <div className="relative">
            <Filter className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
            <select
              value={selectedDex}
              onChange={(e) => setSelectedDex(e.target.value)}
              className="pl-10 pr-8 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-white"
            >
              <option value="all">All DEXes</option>
              {getUniqueDeXes().map(dex => (
                <option key={dex} value={dex}>{getDexDisplayName(dex)}</option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Pool List */}
      <div>
        {filteredPools.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            No pools found matching your criteria
          </div>
        ) : (
          <>
            <div className="divide-y divide-gray-200">
              {getPaginatedPools().map((pool) => (
                <div
                  key={pool.address}
                  onClick={() => onPoolSelect(pool)}
                  className={`p-4 hover:bg-gray-50 cursor-pointer transition-colors ${
                    selectedPool?.address === pool.address ? 'bg-blue-50 border-r-4 border-blue-500' : ''
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <div className="flex items-center space-x-3">
                        <div className="font-medium text-gray-900">
                          {formatTokenPair(pool)}
                        </div>
                        <div className="px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded">
                          {getDexDisplayName(pool.dex)}
                        </div>
                        <div className="text-sm text-gray-500">
                          {formatFee(pool.fee)}
                        </div>
                      </div>
                      <div className="text-sm text-gray-500 mt-1">
                        {formatAddress(pool.address)}
                      </div>
                      {pool.tokens.length >= 2 && (
                        <div className="text-xs text-gray-400 mt-1">
                          {pool.tokens[0].name} / {pool.tokens[1].name}
                        </div>
                      )}
                    </div>
                    <div className="flex items-center space-x-2">
                      <ExternalLink className="h-4 w-4 text-gray-400" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
            
            {/* Pagination */}
            {totalPages > 1 && (
              <Pagination
                currentPage={currentPage}
                totalPages={totalPages}
                onPageChange={handlePageChange}
                totalItems={filteredPools.length}
                itemsPerPage={itemsPerPage}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
}