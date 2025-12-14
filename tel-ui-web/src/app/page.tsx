'use client';

import React, { useState } from 'react';
import { useRouter } from 'next/navigation';
import { TokenSelector } from '@/components/TokenSelector';
import { LiquidityChart } from '@/components/LiquidityChart';
import { StatsSummary } from '@/components/StatsSummary';
import { PoolList } from '@/components/PoolList';
import { useLiquidityData } from '@/hooks/useLiquidityData';
import { Loader2, RefreshCw, AlertCircle, BarChart3, TrendingUp, ArrowLeft, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Pool } from '@/types/api';

export default function HomePage() {
  const router = useRouter();
  const [tokens, setTokens] = useState<{ token0: string; token1: string } | null>(null);
  const [filters, setFilters] = useState<{ chainId?: number; dex?: string }>({});
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [currentView, setCurrentView] = useState<'pools' | 'analysis'>('pools');
  const [priceType, setPriceType] = useState<'wall' | 'current'>('wall');

  const { data, error, isLoading, refresh } = useLiquidityData(
    tokens?.token0 || null,
    tokens?.token1 || null,
    filters,
    priceType
  );

  const handlePoolSelect = (pool: Pool) => {
    setSelectedPool(pool);
    if (pool.tokens.length >= 2) {
      setTokens({ token0: pool.tokens[0].address, token1: pool.tokens[1].address });
      setCurrentView('analysis');
    }
  };

  const handleBackToPools = () => {
    setCurrentView('pools');
    setSelectedPool(null);
    setTokens(null);
  };

  return (
    <div className="min-h-screen bg-[var(--background)]">
      {/* Header */}
      <header className="sticky top-0 z-50 glass border-b border-[var(--border)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-4">
              {currentView === 'analysis' && (
                <Button variant="ghost" size="sm" onClick={handleBackToPools}>
                  <ArrowLeft className="h-4 w-4" />
                  Back
                </Button>
              )}
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg gradient-accent flex items-center justify-center">
                  <BarChart3 className="h-4 w-4 text-white" />
                </div>
                <h1 className="text-xl font-bold text-[var(--foreground)]">tel-on-chain</h1>
                <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-[var(--accent-subtle)] text-[var(--accent)]">
                  Beta
                </span>
              </div>
            </div>
            <div className="flex items-center gap-3">
              {data && currentView === 'analysis' && (
                <>
                  <span className="text-sm text-[var(--foreground-muted)]">
                    {new Date(data.timestamp).toLocaleTimeString()}
                  </span>
                  <Button variant="outline" size="sm" onClick={() => refresh()} isLoading={isLoading}>
                    <RefreshCw className="h-4 w-4" />
                  </Button>
                </>
              )}
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Pool List View */}
        {currentView === 'pools' && (
          <div className="space-y-8 animate-fade-in">
            {/* Hero Section */}
            <div className="text-center py-8">
              <h2 className="text-3xl font-bold text-[var(--foreground)] mb-3">
                Liquidity Pools
              </h2>
              <p className="text-[var(--foreground-muted)] max-w-xl mx-auto mb-6">
                Browse pools and analyze liquidity distribution across DEXes
              </p>

              <Button onClick={() => router.push('/token-aggregate')} variant="outline">
                <TrendingUp className="h-4 w-4" />
                Token Aggregate Analysis
              </Button>
            </div>

            <PoolList onPoolSelect={handlePoolSelect} selectedPool={selectedPool || undefined} />
          </div>
        )}

        {/* Analysis View */}
        {currentView === 'analysis' && (
          <div className="animate-fade-in">
            <TokenSelector
              onTokensChange={(token0, token1) => setTokens({ token0, token1 })}
              onFiltersChange={setFilters}
            />

            {!tokens && (
              <div className="text-center py-16">
                <div className="max-w-md mx-auto">
                  <div className="w-16 h-16 rounded-2xl gradient-accent flex items-center justify-center mx-auto mb-6">
                    <Sparkles className="h-8 w-8 text-white" />
                  </div>
                  <h2 className="text-2xl font-bold text-[var(--foreground)] mb-3">
                    Welcome to tel-on-chain
                  </h2>
                  <p className="text-[var(--foreground-muted)] mb-8">
                    Visualize on-chain liquidity and identify buy/sell walls across multiple DEXes
                  </p>
                  <div className="card p-6">
                    <ul className="text-sm text-[var(--foreground-muted)] space-y-3 text-left">
                      <li className="flex items-center gap-3">
                        <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                        Select a token pair to analyze
                      </li>
                      <li className="flex items-center gap-3">
                        <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                        View liquidity distribution across price levels
                      </li>
                      <li className="flex items-center gap-3">
                        <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                        Identify key support and resistance zones
                      </li>
                    </ul>
                  </div>
                  <div className="mt-8 flex gap-3 justify-center">
                    <Button onClick={handleBackToPools} variant="outline">
                      Browse Pools
                    </Button>
                    <Button onClick={() => router.push('/token-aggregate')}>
                      <TrendingUp className="h-4 w-4" />
                      Token Analysis
                    </Button>
                  </div>
                </div>
              </div>
            )}

            {tokens && (
              <div className="mt-8 space-y-6">
                {/* Selected Pool Info */}
                {selectedPool && (
                  <div className="card p-4 animate-slide-up">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="font-semibold text-[var(--foreground)]">
                          <span className="text-[var(--accent)]">{selectedPool.tokens[0]?.symbol}</span>
                          <span className="text-[var(--foreground-muted)]">/</span>
                          <span className="text-purple-400">{selectedPool.tokens[1]?.symbol}</span>
                        </h3>
                        <p className="text-sm text-[var(--foreground-muted)] mt-1">
                          {selectedPool.dex.replace('_', ' ').toUpperCase()} · {(selectedPool.fee / 10000).toFixed(2)}% fee
                        </p>
                      </div>
                      <code className="text-xs text-[var(--foreground-muted)] font-mono">
                        {selectedPool.address.slice(0, 8)}...{selectedPool.address.slice(-6)}
                      </code>
                    </div>
                  </div>
                )}

                {/* Loading */}
                {isLoading && !data && (
                  <div className="flex items-center justify-center py-16">
                    <div className="text-center">
                      <Loader2 className="h-8 w-8 animate-spin text-[var(--accent)] mx-auto mb-4" />
                      <p className="text-[var(--foreground-muted)]">Loading liquidity data...</p>
                    </div>
                  </div>
                )}

                {/* Error */}
                {error && (
                  <div className="card p-4 border-[var(--danger)]/30 bg-[var(--danger-subtle)]">
                    <div className="flex items-start gap-3">
                      <AlertCircle className="h-5 w-5 text-[var(--danger)] flex-shrink-0 mt-0.5" />
                      <div>
                        <h3 className="font-medium text-[var(--danger)]">Error loading data</h3>
                        <p className="text-sm text-[var(--danger)]/80 mt-1">
                          {error.message || 'Failed to fetch liquidity data'}
                        </p>
                        <Button variant="outline" size="sm" onClick={() => refresh()} className="mt-3">
                          Try Again
                        </Button>
                      </div>
                    </div>
                  </div>
                )}

                {/* Data Display */}
                {data && (
                  <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 animate-slide-up">
                    <div className="lg:col-span-2">
                      <LiquidityChart
                        data={data.chartData}
                        currentPrice={data.price}
                        token0Symbol={data.token0.symbol}
                        token1Symbol={data.token1.symbol}
                        priceType={priceType}
                        onPriceTypeChange={setPriceType}
                      />
                    </div>
                    <div className="lg:col-span-1">
                      <StatsSummary
                        totalBuyLiquidity={data.totalBuyLiquidity}
                        totalSellLiquidity={data.totalSellLiquidity}
                        strongestBuyWall={data.strongestBuyWall}
                        strongestSellWall={data.strongestSellWall}
                        currentPrice={data.price}
                        token0Symbol={data.token0.symbol}
                        token1Symbol={data.token1.symbol}
                        chainId={filters.chainId || 1}
                      />
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="border-t border-[var(--border)] mt-16">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <p className="text-center text-sm text-[var(--foreground-muted)]">
            <a
              href="https://github.com/decipherhub/tel-on-chain"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--accent)] transition-colors"
            >
              tel-on-chain
            </a>
            {' '}· On-chain liquidity visualization for DeFi
          </p>
        </div>
      </footer>
    </div>
  );
}
