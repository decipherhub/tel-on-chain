'use client';

import React, { useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, Search, TrendingUp, AlertCircle, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { StatsSummary } from '@/components/StatsSummary';
import { LiquidityChart } from '@/components/LiquidityChart';
import { useTokenAggregateData } from '@/hooks/useTokenAggregateData';
import { formatNumber } from '@/lib/utils';

export default function TokenAggregatePage() {
  const router = useRouter();
  const [tokenAddress, setTokenAddress] = useState('');
  const [submittedToken, setSubmittedToken] = useState<string | null>(null);
  const [chainId] = useState(1); // Default to Ethereum mainnet
  const [priceType, setPriceType] = useState<'wall' | 'current'>('current');

  const { data, loading, error } = useTokenAggregateData({
    tokenAddress: submittedToken || undefined,
    dex: 'all',
    chainId,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (tokenAddress.trim()) {
      setSubmittedToken(tokenAddress.trim());
    }
  };

  const handleClear = () => {
    setTokenAddress('');
    setSubmittedToken(null);
  };

  // Process aggregate data for chart visualization
  const chartData = data?.price_levels?.filter(level => 
    level.lower_price != null && level.upper_price != null
  ).map(level => {
    const price = (level.lower_price + level.upper_price) / 2;
    return {
      priceRange: `${level.lower_price.toFixed(4)} - ${level.upper_price.toFixed(4)}`,
      price,
      buyLiquidity: level.side === 'Buy' ? level.token1_liquidity : 0,
      sellLiquidity: level.side === 'Sell' ? level.token1_liquidity : 0,
      type: level.side.toLowerCase() as 'buy' | 'sell',
      dexSources: {},
    };
  }) || [];

  // Example token addresses for quick testing
  const exampleTokens = [
    { name: 'WETH', address: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' },
    { name: 'LINK', address: '0x514910771AF9Ca656af840dff83E8264EcF986CA' },
    { name: 'UNI', address: '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984' },
    { name: 'PEPE', address: '0x6982508145454ce325ddbe47a25d4ec3d2311933' },
  ];

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white shadow-sm border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center">
              <Button
                variant="outline"
                size="sm"
                onClick={() => router.push('/')}
                className="mr-4"
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back to Home
              </Button>
              <div className="flex items-center">
                <h1 className="text-2xl font-bold text-gray-900">Token Aggregate Analysis</h1>
                <span className="ml-3 px-2 py-1 bg-purple-100 text-purple-800 text-xs font-medium rounded">
                  Cross-DEX
                </span>
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Search Section */}
        <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200 mb-8">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            🔍 Analyze Token Liquidity (Across Major Pairs)
          </h2>

          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="flex gap-4">
              <div className="flex-1">
                <Input
                  type="text"
                  placeholder="Enter token address (0x...)"
                  value={tokenAddress}
                  onChange={(e) => setTokenAddress(e.target.value)}
                  className="w-full"
                />
              </div>
              <Button type="submit" disabled={!tokenAddress.trim() || loading}>
                <Search className="h-4 w-4 mr-2" />
                Analyze
              </Button>
              {submittedToken && (
                <Button type="button" variant="outline" onClick={handleClear}>
                  Clear
                </Button>
              )}
            </div>
          </form>

          {/* Example Tokens */}
          <div className="mt-4">
            <p className="text-sm text-gray-500 mb-2">Try these example tokens:</p>
            <div className="flex flex-wrap gap-2">
              {exampleTokens.map((token) => (
                <Button
                  key={token.address}
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setTokenAddress(token.address);
                    setSubmittedToken(token.address);
                  }}
                >
                  {token.name}
                </Button>
              ))}
            </div>
          </div>
        </div>

        {/* Results Section */}
        {!submittedToken && (
          <div className="text-center py-12">
            <div className="max-w-md mx-auto">
              <TrendingUp className="h-16 w-16 text-gray-400 mx-auto mb-4" />
              <h3 className="text-xl font-semibold text-gray-900 mb-4">
                Ready to Analyze Token Liquidity
              </h3>
              <p className="text-gray-600 mb-6">
                Enter a token address above to see comprehensive liquidity analysis across all major DEX pairs.
              </p>
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                <div className="flex items-start">
                  <AlertCircle className="h-5 w-5 text-blue-600 mt-0.5 mr-3" />
                  <div className="text-sm text-blue-800">
                    <p className="font-medium mb-1">What you&apos;ll see:</p>
                    <ul className="list-disc list-inside space-y-1 text-left">
                      <li>Total liquidity across WETH, USDC, USDT, DAI, WBTC pairs</li>
                      <li>Buy vs sell liquidity distribution</li>
                      <li>Strongest support and resistance levels</li>
                      <li>Aggregated market depth analysis</li>
                    </ul>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="text-center">
              <Loader2 className="h-8 w-8 animate-spin text-purple-600 mx-auto mb-4" />
              <p className="text-gray-600">Analyzing token liquidity across major pairs...</p>
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
                  Error loading aggregate data
                </h3>
                <p className="text-sm text-red-700 mt-1">
                  {error || 'Failed to fetch token aggregate liquidity. Please check the token address and try again.'}
                </p>
              </div>
            </div>
            <div className="mt-4">
              <Button variant="outline" size="sm" onClick={() => setSubmittedToken(submittedToken)}>
                Try Again
              </Button>
            </div>
          </div>
        )}

        {/* Data Display */}
        {data && submittedToken && (
          <div className="space-y-8">
            {/* Token Info Header */}
            <div className="bg-white border border-gray-200 rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-xl font-bold text-gray-900">
                    <span className="text-blue-600">{data.token0.symbol}</span>
                  </h3>
                  <p className="text-xs text-gray-400 mt-1 font-mono">
                    Token Address: {data.token0.address}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-sm text-gray-500">Current Price</p>
                  <p className="text-2xl font-bold text-gray-900">
                    ${data.current_price.toFixed(4)}
                  </p>
                  <p className="text-xs text-gray-500">
                    Updated: {new Date(data.timestamp).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>

            {/* Aggregate Stats */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
              {/* Liquidity Overview */}
              <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
                <h3 className="text-lg font-semibold text-gray-900 mb-4">
                  Liquidity Overview
                </h3>
                <div className="space-y-4">
                  <div className="flex justify-between items-center">
                    <span className="text-gray-600">Total Price Levels</span>
                    <span className="font-semibold">{data.price_levels.length}</span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-gray-600">Buy Levels</span>
                    <span className="font-semibold text-green-600">
                      {data.price_levels.filter(level => level.side === 'Buy').length}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-gray-600">Sell Levels</span>
                    <span className="font-semibold text-red-600">
                      {data.price_levels.filter(level => level.side === 'Sell').length}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-gray-600">Total Liquidity</span>
                    <span className="font-bold">
                      {formatNumber(
                        data.price_levels.reduce((sum, level) => sum + level.token1_liquidity, 0),
                        { compact: true }
                      )} {data.token1.symbol}
                    </span>
                  </div>
                </div>
              </div>

              {/* Stats Summary Component */}
              <StatsSummary
                totalBuyLiquidity={0} // Will be calculated by component
                totalSellLiquidity={0} // Will be calculated by component
                currentPrice={data.current_price}
                token0Symbol={data.token0.symbol}
                token1Symbol={data.token1.symbol}
                mode="aggregate"
                tokenAddress={submittedToken}
                chainId={chainId}
              />
            </div>

            {/* Liquidity Chart */}
            <div className="mt-8">
              <LiquidityChart
                data={chartData}
                currentPrice={data.current_price}
                token0Symbol={data.token0.symbol}
                token1Symbol={data.token1.symbol}
                priceType={priceType}
                onPriceTypeChange={setPriceType}
              />
            </div>
          </div>
        )}
      </main>
    </div>
  );
}