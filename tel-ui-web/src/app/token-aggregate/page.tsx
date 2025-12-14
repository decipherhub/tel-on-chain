'use client';

import React, { useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, Search, TrendingUp, AlertCircle, Loader2, BarChart3 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { StatsSummary } from '@/components/StatsSummary';
import { LiquidityChart } from '@/components/LiquidityChart';
import { useTokenAggregateData } from '@/hooks/useTokenAggregateData';

const EXAMPLE_TOKENS = [
  { name: 'WETH', address: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2' },
  { name: 'LINK', address: '0x514910771AF9Ca656af840dff83E8264EcF986CA' },
  { name: 'UNI', address: '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984' },
  { name: 'PEPE', address: '0x6982508145454ce325ddbe47a25d4ec3d2311933' },
  { name: 'WBTC', address: '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599' },
];

export default function TokenAggregatePage() {
  const router = useRouter();
  const [tokenAddress, setTokenAddress] = useState('');
  const [submittedToken, setSubmittedToken] = useState<string | null>(null);
  const [priceType, setPriceType] = useState<'wall' | 'current'>('current');

  const { data, loading, error } = useTokenAggregateData({
    tokenAddress: submittedToken || undefined,
    dex: 'all',
    chainId: 1,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (tokenAddress.trim()) setSubmittedToken(tokenAddress.trim());
  };

  const chartData = data?.price_levels?.filter(level =>
    level.lower_price != null && level.upper_price != null
  ).map(level => ({
    priceRange: `${level.lower_price.toFixed(4)} - ${level.upper_price.toFixed(4)}`,
    price: (level.lower_price + level.upper_price) / 2,
    buyLiquidity: level.side === 'Buy' ? level.token1_liquidity : 0,
    sellLiquidity: level.side === 'Sell' ? level.token1_liquidity : 0,
    type: level.side.toLowerCase() as 'buy' | 'sell',
    dexSources: {},
  })) || [];

  return (
    <div className="min-h-screen bg-[var(--background)]">
      {/* Header */}
      <header className="sticky top-0 z-50 glass border-b border-[var(--border)]">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-4">
              <Button variant="ghost" size="sm" onClick={() => router.push('/')}>
                <ArrowLeft className="h-4 w-4" />
                Home
              </Button>
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg gradient-accent flex items-center justify-center">
                  <BarChart3 className="h-4 w-4 text-white" />
                </div>
                <h1 className="text-xl font-bold text-[var(--foreground)]">Token Aggregate</h1>
                <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-purple-500/10 text-purple-400">
                  Cross-DEX
                </span>
              </div>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Search Section */}
        <div className="card p-6 mb-8 animate-fade-in">
          <h2 className="text-lg font-semibold text-[var(--foreground)] mb-4">
            Analyze Token Liquidity
          </h2>

          <form onSubmit={handleSubmit} className="flex gap-3 mb-4">
            <div className="flex-1">
              <Input
                placeholder="Enter token address (0x...)"
                value={tokenAddress}
                onChange={(e) => setTokenAddress(e.target.value)}
              />
            </div>
            <Button type="submit" disabled={!tokenAddress.trim() || loading}>
              <Search className="h-4 w-4" />
              Analyze
            </Button>
            {submittedToken && (
              <Button type="button" variant="outline" onClick={() => { setTokenAddress(''); setSubmittedToken(null); }}>
                Clear
              </Button>
            )}
          </form>

          <div className="flex flex-wrap gap-2">
            <span className="text-sm text-[var(--foreground-muted)]">Try:</span>
            {EXAMPLE_TOKENS.map((token) => (
              <Button
                key={token.address}
                variant="ghost"
                size="sm"
                onClick={() => { setTokenAddress(token.address); setSubmittedToken(token.address); }}
              >
                {token.name}
              </Button>
            ))}
          </div>
        </div>

        {/* Empty State */}
        {!submittedToken && (
          <div className="text-center py-16 animate-fade-in">
            <div className="w-16 h-16 rounded-2xl gradient-accent flex items-center justify-center mx-auto mb-6">
              <TrendingUp className="h-8 w-8 text-white" />
            </div>
            <h3 className="text-xl font-bold text-[var(--foreground)] mb-3">
              Token Aggregate Analysis
            </h3>
            <p className="text-[var(--foreground-muted)] max-w-md mx-auto mb-6">
              Analyze any token&apos;s liquidity across all major DEX pairs in one view
            </p>
            <div className="card p-5 max-w-md mx-auto">
              <ul className="text-sm text-[var(--foreground-muted)] space-y-2 text-left">
                <li className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                  Total liquidity across WETH, USDC, USDT, DAI, WBTC pairs
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                  Buy vs sell liquidity distribution
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1.5 h-1.5 rounded-full bg-[var(--accent)]" />
                  Strongest support and resistance levels
                </li>
              </ul>
            </div>
          </div>
        )}

        {/* Loading */}
        {loading && (
          <div className="flex items-center justify-center py-16">
            <Loader2 className="h-8 w-8 animate-spin text-[var(--accent)] mr-3" />
            <span className="text-[var(--foreground-muted)]">Analyzing token liquidity...</span>
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="card p-4 border-[var(--danger)]/30 bg-[var(--danger-subtle)] mb-8">
            <div className="flex items-start gap-3">
              <AlertCircle className="h-5 w-5 text-[var(--danger)] flex-shrink-0" />
              <div>
                <h3 className="font-medium text-[var(--danger)]">Error loading data</h3>
                <p className="text-sm text-[var(--danger)]/80 mt-1">{error}</p>
                <Button variant="outline" size="sm" onClick={() => setSubmittedToken(submittedToken)} className="mt-3">
                  Try Again
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* Results */}
        {data && submittedToken && (
          <div className="space-y-6 animate-slide-up">
            {/* Token Info */}
            <div className="card p-5">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-xl font-bold text-[var(--accent)]">{data.token0.symbol}</h3>
                  <code className="text-xs text-[var(--foreground-muted)] font-mono mt-1">
                    {data.token0.address}
                  </code>
                </div>
                <span className="text-xs text-[var(--foreground-muted)]">
                  {new Date(data.timestamp).toLocaleString()}
                </span>
              </div>
            </div>

            {/* Charts and Stats */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <LiquidityChart
                data={chartData}
                currentPrice={data.current_price}
                token0Symbol={data.token0.symbol}
                token1Symbol={data.token1.symbol}
                priceType={priceType}
                onPriceTypeChange={setPriceType}
              />
              <StatsSummary
                totalBuyLiquidity={0}
                totalSellLiquidity={0}
                currentPrice={data.current_price}
                token0Symbol={data.token0.symbol}
                token1Symbol={data.token1.symbol}
                mode="aggregate"
                tokenAddress={submittedToken}
                chainId={1}
              />
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
