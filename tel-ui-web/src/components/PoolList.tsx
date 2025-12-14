'use client';

import React, { useState, useEffect } from 'react';
import { Pool } from '@/types/api';
import { usePoolData } from '@/hooks/usePoolData';
import { Loader2, Search, ExternalLink, Filter } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Pagination } from '@/components/ui/Pagination';

interface PoolListProps {
  onPoolSelect: (pool: Pool) => void;
  selectedPool?: Pool;
}

const DEX_NAMES: Record<string, string> = {
  uniswap_v2: 'Uniswap V2',
  uniswap_v3: 'Uniswap V3',
  sushiswap: 'SushiSwap',
};

export function PoolList({ onPoolSelect, selectedPool }: PoolListProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedDex, setSelectedDex] = useState<string>('all');
  const [currentPage, setCurrentPage] = useState(1);
  const itemsPerPage = 100;

  const { pools, loading, error, refresh } = usePoolData({
    chainId: 1,
    dex: selectedDex === 'all' ? undefined : selectedDex,
    page: currentPage,
    limit: itemsPerPage
  });

  const filteredPools = searchTerm
    ? pools.filter(pool =>
      pool.tokens?.some(token =>
        token.symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
        token.name.toLowerCase().includes(searchTerm.toLowerCase())
      ) || pool.address.toLowerCase().includes(searchTerm.toLowerCase())
    )
    : pools;

  useEffect(() => {
    setCurrentPage(1);
  }, [selectedDex]);

  const formatTokenPair = (pool: Pool) => {
    if (!pool.tokens || pool.tokens.length < 2) return 'Unknown';
    return `${pool.tokens[0].symbol}/${pool.tokens[1].symbol}`;
  };

  if (loading) {
    return (
      <div className="card p-8">
        <div className="flex items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-[var(--accent)] mr-3" />
          <span className="text-[var(--foreground-muted)]">Loading pools...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="card p-8 text-center">
        <p className="text-[var(--danger)] mb-4">Error loading pools</p>
        <Button onClick={refresh} variant="outline" size="sm">Try Again</Button>
      </div>
    );
  }

  return (
    <div className="card">
      {/* Header */}
      <div className="p-5 border-b border-[var(--border)]">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-[var(--foreground)]">Available Pools</h2>
          <span className="text-sm text-[var(--foreground-muted)]">{filteredPools.length} pools</span>
        </div>

        {/* Search and Filter */}
        <div className="flex gap-3">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--foreground-muted)]" />
            <input
              type="text"
              placeholder="Search by symbol, name, or address..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full pl-10 pr-4 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--border)] text-[var(--foreground)] placeholder:text-[var(--foreground-muted)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
          <div className="relative">
            <Filter className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-[var(--foreground-muted)]" />
            <select
              value={selectedDex}
              onChange={(e) => setSelectedDex(e.target.value)}
              className="pl-10 pr-8 py-2 text-sm rounded-lg bg-[var(--surface)] border border-[var(--border)] text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            >
              <option value="all">All DEXes</option>
              <option value="uniswap_v2">Uniswap V2</option>
              <option value="uniswap_v3">Uniswap V3</option>
              <option value="sushiswap">SushiSwap</option>
            </select>
          </div>
        </div>
      </div>

      {/* Pool List */}
      {filteredPools.length === 0 ? (
        <div className="p-8 text-center text-[var(--foreground-muted)]">
          No pools found
        </div>
      ) : (
        <>
          <div className="divide-y divide-[var(--border)]">
            {filteredPools.map((pool) => (
              <div
                key={pool.address}
                onClick={() => onPoolSelect(pool)}
                className={`p-4 cursor-pointer transition-all hover:bg-[var(--surface)] ${selectedPool?.address === pool.address
                    ? 'bg-[var(--accent-subtle)] border-l-2 border-l-[var(--accent)]'
                    : ''
                  }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3">
                      <span className="font-medium text-[var(--foreground)]">
                        {formatTokenPair(pool)}
                      </span>
                      <span className="px-2 py-0.5 text-xs rounded bg-[var(--surface)] text-[var(--foreground-muted)]">
                        {DEX_NAMES[pool.dex] || pool.dex}
                      </span>
                      <span className="text-xs text-[var(--foreground-muted)]">
                        {(pool.fee / 10000).toFixed(2)}%
                      </span>
                    </div>
                    <div className="mt-1 text-xs text-[var(--foreground-muted)] font-mono truncate">
                      {pool.address.slice(0, 8)}...{pool.address.slice(-6)}
                    </div>
                  </div>
                  <ExternalLink className="h-4 w-4 text-[var(--foreground-muted)]" />
                </div>
              </div>
            ))}
          </div>

          {pools.length === itemsPerPage && (
            <Pagination
              currentPage={currentPage}
              totalPages={currentPage + 1}
              onPageChange={setCurrentPage}
              totalItems={pools.length}
              itemsPerPage={itemsPerPage}
            />
          )}
        </>
      )}
    </div>
  );
}
