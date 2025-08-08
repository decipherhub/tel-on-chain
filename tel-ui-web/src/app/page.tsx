'use client';

import React, { useState } from 'react';
import { useRouter } from 'next/navigation';
import { TokenSelector } from '@/components/TokenSelector';
import { LiquidityChart } from '@/components/LiquidityChart';
import { StatsSummary } from '@/components/StatsSummary';
import { PoolList } from '@/components/PoolList';
import { useLiquidityData } from '@/hooks/useLiquidityData';
import { Loader2, RefreshCw, AlertCircle, List, TrendingUp, ArrowLeft } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Pool } from '@/types/api';

export default function HomePage() {
  const router = useRouter();
  const [tokens, setTokens] = useState<{ token0: string; token1: string } | null>(null);
  const [filters, setFilters] = useState<{ chainId?: number; dex?: string }>({});
  const [selectedPool, setSelectedPool] = useState<Pool | null>(null);
  const [currentView, setCurrentView] = useState<'pools' | 'analysis'>('pools');
  const [priceType, setPriceType] = useState<'wall' | 'current'>('wall');
  const [statsMode] = useState<'pair' | 'aggregate'>('pair');

  const { data, error, isLoading, refresh } = useLiquidityData(
    tokens?.token0 || null,
    tokens?.token1 || null,
    filters,
    priceType
  );

  const handleTokensChange = (token0: string, token1: string) => {
    setTokens({ token0, token1 });
  };

  const handleFiltersChange = (newFilters: { chainId?: number; dex?: string }) => {
    setFilters(newFilters);
  };

  const handleRefresh = () => {
    refresh();
  };

  const handlePoolSelect = (pool: Pool) => {
    setSelectedPool(pool);
    if (pool.tokens.length >= 2) {
      setTokens({
        token0: pool.tokens[0].address,
        token1: pool.tokens[1].address,
      });
      setCurrentView('analysis');
    }
  };

  const handleBackToPools = () => {
    setCurrentView('pools');
    setSelectedPool(null);
    setTokens(null);
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white shadow-sm border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center">
              {currentView === 'analysis' && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleBackToPools}
                  className="mr-4"
                >
                  <ArrowLeft className="h-4 w-4 mr-2" />
                  Back to Pools
                </Button>
              )}
              <h1 className="text-2xl font-bold text-gray-900">tel-on-chain</h1>
              <span className="ml-3 px-2 py-1 bg-blue-100 text-blue-800 text-xs font-medium rounded">
                Beta
              </span>
            </div>
            <div className="flex items-center space-x-4">
              
              {data && currentView === 'analysis' && (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleRefresh}
                    isLoading={isLoading}
                  >
                    <RefreshCw className="h-4 w-4 mr-2" />
                    Refresh
                  </Button>
                </>
              )}
              <div className="text-sm text-gray-500">
                {data && currentView === 'analysis' && `Last updated: ${new Date(data.timestamp).toLocaleTimeString()}`}
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Pool List View */}
        {currentView === 'pools' && (
          <div className="space-y-6">
            <div className="text-center py-6">
              <h2 className="text-2xl font-bold text-gray-900 mb-4">
                Available Liquidity Pools
              </h2>
              <p className="text-gray-600 max-w-2xl mx-auto mb-4">
                Browse and select from available liquidity pools across multiple DEXes. 
                Click on any pool to analyze its liquidity distribution.
              </p>
              
              {/* Quick Access to Token Aggregate */}
              <div className="inline-flex items-center gap-4 bg-purple-50 border border-purple-200 rounded-lg p-3">
                <div className="text-sm text-purple-800">
                  <span className="font-medium">Looking for token-wide analysis?</span>
                </div>
                <Button 
                  onClick={() => router.push('/token-aggregate')}
                  size="sm"
                  variant="primary"
                >
                  <TrendingUp className="h-4 w-4 mr-2" />
                  Token Aggregate
                </Button>
              </div>
            </div>
            
            <PoolList 
              onPoolSelect={handlePoolSelect}
              selectedPool={selectedPool || undefined}
            />
          </div>
        )}

        {/* Analysis View */}
        {currentView === 'analysis' && (
          <>
            {/* Token Selector */}
            <div className="mb-8">
              <TokenSelector
                onTokensChange={handleTokensChange}
                onFiltersChange={handleFiltersChange}
              />
            </div>

            {/* Content Area */}
            {!tokens && (
              <div className="text-center py-12">
                <div className="max-w-md mx-auto">
                  <h2 className="text-xl font-semibold text-gray-900 mb-4">
                    Welcome to tel-on-chain
                  </h2>
                  <p className="text-gray-600 mb-6">
                    Visualize on-chain liquidity and identify buy/sell walls across multiple DEXes.
                    Enter token addresses above to get started, or browse available pools.
                  </p>
                  <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                    <div className="flex items-start">
                      <AlertCircle className="h-5 w-5 text-blue-600 mt-0.5 mr-3" />
                      <div className="text-sm text-blue-800">
                        <p className="font-medium mb-1">How it works:</p>
                        <ul className="list-disc list-inside space-y-1 text-left">
                          <li>Select a token pair to analyze</li>
                          <li>View liquidity distribution across price levels</li>
                          <li>Identify key support and resistance zones</li>
                          <li>Compare liquidity across different DEXes</li>
                        </ul>
                      </div>
                    </div>
                  </div>
                  <div className="mt-6 space-y-4">
                    <div className="flex flex-col sm:flex-row gap-3">
                      <Button 
                        onClick={handleBackToPools}
                        variant="outline"
                      >
                        <List className="h-4 w-4 mr-2" />
                        Browse Available Pools
                      </Button>
                      <Button 
                        onClick={() => router.push('/token-aggregate')}
                        variant="primary"
                      >
                        <TrendingUp className="h-4 w-4 mr-2" />
                        Token Aggregate Analysis
                      </Button>
                    </div>
                    
                    {/* Token Aggregate Feature Card */}
                    <div className="bg-purple-50 border border-purple-200 rounded-lg p-4 mt-4">
                      <div className="flex items-start">
                        <TrendingUp className="h-5 w-5 text-purple-600 mt-0.5 mr-3" />
                        <div className="text-sm text-purple-800">
                          <p className="font-medium mb-1">🚀 New: Token Aggregate Analysis</p>
                          <p className="mb-2">
                            Analyze any token&apos;s liquidity across all major trading pairs (WETH, USDC, USDT, DAI, WBTC) in one view.
                          </p>
                          <ul className="list-disc list-inside space-y-1 text-left text-xs">
                            <li>Cross-DEX liquidity aggregation</li>
                            <li>Comprehensive market depth analysis</li>
                            <li>Single token focus instead of pair analysis</li>
                          </ul>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {tokens && (
              <>
                {/* Selected Pool Info */}
                {selectedPool && (
                  <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="font-medium text-gray-900">
                          <span className="text-blue-600">{selectedPool.tokens[0]?.symbol}</span>/<span className="text-purple-600">{selectedPool.tokens[1]?.symbol}</span>
                        </h3>
                        <p className="text-sm text-gray-500">
                          {selectedPool.dex.replace('_', ' ').toUpperCase()} • Fee: {(selectedPool.fee / 10000).toFixed(2)}%
                        </p>
                        <p className="text-xs text-gray-400 mt-1 font-mono">
                          Pool Address: {selectedPool.address}
                        </p>
                      </div>
                    </div>
                  </div>
                )}

                {/* Loading State */}
                {isLoading && !data && (
                  <div className="flex items-center justify-center py-12">
                    <div className="text-center">
                      <Loader2 className="h-8 w-8 animate-spin text-blue-600 mx-auto mb-4" />
                      <p className="text-gray-600">Loading liquidity data...</p>
                    </div>
                  </div>
                )}

                {/* Error State */}
                {error && (
                  <div className="bg-red-50 border border-red-200 rounded-lg p-4 mb-8">
                    <div className="flex items-center">
                      <AlertCircle className="h-5 w-5 text-red-600 mr-3" />
                      <div>
                        <h3 className="text-sm font-medium text-red-800">
                          Error loading data
                        </h3>
                        <p className="text-sm text-red-700 mt-1">
                          {error.message || 'Failed to fetch liquidity data. Please try again.'}
                        </p>
                      </div>
                    </div>
                    <div className="mt-4">
                      <Button variant="outline" size="sm" onClick={handleRefresh}>
                        Try Again
                      </Button>
                    </div>
                  </div>
                )}

                {/* Data Display */}
                {data && (
                  <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    {/* Chart - Takes 2/3 of the width on large screens */}
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

                    {/* Stats Summary - Takes 1/3 of the width on large screens */}
                    <div className="lg:col-span-1">
                      <StatsSummary
                        totalBuyLiquidity={data.totalBuyLiquidity}
                        totalSellLiquidity={data.totalSellLiquidity}
                        strongestBuyWall={data.strongestBuyWall}
                        strongestSellWall={data.strongestSellWall}
                        currentPrice={data.price}
                        token0Symbol={data.token0.symbol}
                        token1Symbol={data.token1.symbol}
                        mode={statsMode}
                        tokenAddress={statsMode === 'aggregate' ? tokens?.token0 : undefined}
                        chainId={filters.chainId || 1}
                      />
                    </div>
                  </div>
                )}
              </>
            )}
          </>
        )}
      </main>

      {/* Footer */}
      <footer className="bg-white border-t border-gray-200 mt-16">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          <div className="text-center text-sm text-gray-500">
            <p>
              Built with{' '}
              <a
                href="https://github.com/decipherhub/tel-on-chain"
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:text-blue-800"
              >
                tel-on-chain
              </a>{' '}
              - On-chain liquidity visualization for DeFi
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
