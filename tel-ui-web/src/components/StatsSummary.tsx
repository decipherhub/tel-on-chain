'use client';

import React from 'react';
import { formatNumber, formatPrice } from '@/lib/utils';
import { LiquidityWall } from '@/types/api';
import { TrendingUp, TrendingDown, DollarSign, Shield } from 'lucide-react';
import { useTokenAggregateData } from '@/hooks/useTokenAggregateData';

interface StatsSummaryProps {
  totalBuyLiquidity: number;
  totalSellLiquidity: number;
  strongestBuyWall?: LiquidityWall;
  strongestSellWall?: LiquidityWall;
  currentPrice: number;
  token0Symbol: string;
  token1Symbol: string;
  mode?: 'pair' | 'aggregate';
  tokenAddress?: string;
  chainId?: number;
}

export function StatsSummary({
  totalBuyLiquidity,
  totalSellLiquidity,
  strongestBuyWall,
  strongestSellWall,
  currentPrice,
  token0Symbol,
  token1Symbol,
  mode = 'pair',
  tokenAddress,
  chainId = 1,
}: StatsSummaryProps) {
  const { data: aggregateData } = useTokenAggregateData({
    tokenAddress: mode === 'aggregate' ? tokenAddress : undefined,
    dex: 'all',
    chainId,
  });

  const displayData = React.useMemo(() => {
    if (mode === 'aggregate' && aggregateData) {
      const buyLevels = aggregateData.price_levels.filter(level => level.side === 'Buy');
      const sellLevels = aggregateData.price_levels.filter(level => level.side === 'Sell');

      const aggregateBuyLiquidity = buyLevels.reduce((sum, level) => sum + level.token1_liquidity, 0);
      const aggregateSellLiquidity = sellLevels.reduce((sum, level) => sum + level.token1_liquidity, 0);

      const strongestBuy = buyLevels.length > 0 ? buyLevels.reduce((strongest, level) =>
        level.token1_liquidity > strongest.token1_liquidity ? level : strongest,
        buyLevels[0]
      ) : undefined;
      const strongestSell = sellLevels.length > 0 ? sellLevels.reduce((strongest, level) =>
        level.token1_liquidity > strongest.token1_liquidity ? level : strongest,
        sellLevels[0]
      ) : undefined;

      return {
        totalBuyLiquidity: aggregateBuyLiquidity,
        totalSellLiquidity: aggregateSellLiquidity,
        strongestBuyLevel: strongestBuy,
        strongestSellLevel: strongestSell,
        currentPrice: aggregateData.current_price,
        token0Symbol: aggregateData.token0.symbol,
        token1Symbol: aggregateData.token1.symbol,
        isAggregate: true,
      };
    }

    return {
      totalBuyLiquidity,
      totalSellLiquidity,
      strongestBuyWall,
      strongestSellWall,
      currentPrice,
      token0Symbol,
      token1Symbol,
      isAggregate: false,
    };
  }, [mode, aggregateData, totalBuyLiquidity, totalSellLiquidity, strongestBuyWall, strongestSellWall, currentPrice, token0Symbol, token1Symbol]);

  const buyToSellRatio = displayData.totalSellLiquidity > 0 ? displayData.totalBuyLiquidity / displayData.totalSellLiquidity : 0;
  const totalLiquidity = displayData.totalBuyLiquidity + displayData.totalSellLiquidity;
  const buyPercentage = totalLiquidity > 0 ? (displayData.totalBuyLiquidity / totalLiquidity) * 100 : 0;

  return (
    <div className="space-y-4">
      {/* Overview Card */}
      <div className="card p-5">
        <h3 className="text-sm font-medium text-[var(--foreground-muted)] mb-4">Overview</h3>

        <div className="grid grid-cols-2 gap-4 mb-5">
          <div className="text-center p-3 rounded-lg bg-[var(--surface)]">
            <DollarSign className="h-4 w-4 text-[var(--accent)] mx-auto mb-1" />
            <p className="text-lg font-bold text-[var(--foreground)]">
              {formatPrice(displayData.currentPrice, displayData.token1Symbol)}
            </p>
            <p className="text-xs text-[var(--foreground-muted)]">Current Price</p>
          </div>
          <div className="text-center p-3 rounded-lg bg-[var(--surface)]">
            <Shield className="h-4 w-4 text-purple-400 mx-auto mb-1" />
            <p className="text-lg font-bold text-[var(--foreground)]">
              {formatNumber(totalLiquidity, { compact: true })}
            </p>
            <p className="text-xs text-[var(--foreground-muted)]">Total Liquidity</p>
          </div>
        </div>

        {/* Buy/Sell Breakdown */}
        <div className="space-y-3">
          <div className="flex items-center justify-between p-3 rounded-lg bg-[var(--success-subtle)]">
            <div className="flex items-center gap-2">
              <TrendingUp className="h-4 w-4 text-[var(--success)]" />
              <span className="text-sm font-medium text-[var(--success)]">Buy</span>
            </div>
            <div className="text-right">
              <p className="text-sm font-bold text-[var(--success)]">
                {formatNumber(displayData.totalBuyLiquidity, { compact: true })}
              </p>
              <p className="text-xs text-[var(--success)]/70">{buyPercentage.toFixed(1)}%</p>
            </div>
          </div>

          <div className="flex items-center justify-between p-3 rounded-lg bg-[var(--danger-subtle)]">
            <div className="flex items-center gap-2">
              <TrendingDown className="h-4 w-4 text-[var(--danger)]" />
              <span className="text-sm font-medium text-[var(--danger)]">Sell</span>
            </div>
            <div className="text-right">
              <p className="text-sm font-bold text-[var(--danger)]">
                {formatNumber(displayData.totalSellLiquidity, { compact: true })}
              </p>
              <p className="text-xs text-[var(--danger)]/70">{(100 - buyPercentage).toFixed(1)}%</p>
            </div>
          </div>
        </div>

        {/* Ratio Bar */}
        <div className="mt-4 p-3 rounded-lg bg-[var(--surface)]">
          <div className="flex justify-between items-center mb-2">
            <span className="text-xs text-[var(--foreground-muted)]">Buy/Sell Ratio</span>
            <span className="text-sm font-bold text-[var(--foreground)]">{buyToSellRatio.toFixed(2)}:1</span>
          </div>
          <div className="w-full h-1.5 rounded-full bg-[var(--danger)]/30 overflow-hidden">
            <div
              className="h-full rounded-full bg-[var(--success)] transition-all duration-500"
              style={{ width: `${buyPercentage}%` }}
            />
          </div>
        </div>
      </div>

      {/* Strongest Walls Card */}
      <div className="card p-5">
        <h3 className="text-sm font-medium text-[var(--foreground-muted)] mb-4">Strongest Walls</h3>

        <div className="space-y-3">
          {((displayData.isAggregate && displayData.strongestBuyLevel) || (!displayData.isAggregate && displayData.strongestBuyWall)) && (
            <div className="p-3 rounded-lg border border-[var(--success)]/20 bg-[var(--success-subtle)]">
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-[var(--success)]">Buy Wall</span>
                <span className="text-sm font-bold text-[var(--success)]">
                  {displayData.isAggregate
                    ? formatNumber(displayData.strongestBuyLevel!.token1_liquidity, { compact: true })
                    : formatNumber(displayData.strongestBuyWall!.liquidity_value, { compact: true })}
                </span>
              </div>
              <p className="text-xs text-[var(--foreground-muted)]">
                {displayData.isAggregate
                  ? `${formatPrice(displayData.strongestBuyLevel!.lower_price, displayData.token1Symbol)} - ${formatPrice(displayData.strongestBuyLevel!.upper_price, displayData.token1Symbol)}`
                  : `${formatPrice(displayData.strongestBuyWall!.price_lower, displayData.token1Symbol)} - ${formatPrice(displayData.strongestBuyWall!.price_upper, displayData.token1Symbol)}`}
              </p>
            </div>
          )}

          {((displayData.isAggregate && displayData.strongestSellLevel) || (!displayData.isAggregate && displayData.strongestSellWall)) && (
            <div className="p-3 rounded-lg border border-[var(--danger)]/20 bg-[var(--danger-subtle)]">
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-[var(--danger)]">Sell Wall</span>
                <span className="text-sm font-bold text-[var(--danger)]">
                  {displayData.isAggregate
                    ? formatNumber(displayData.strongestSellLevel!.token1_liquidity, { compact: true })
                    : formatNumber(displayData.strongestSellWall!.liquidity_value, { compact: true })}
                </span>
              </div>
              <p className="text-xs text-[var(--foreground-muted)]">
                {displayData.isAggregate
                  ? `${formatPrice(displayData.strongestSellLevel!.lower_price, displayData.token1Symbol)} - ${formatPrice(displayData.strongestSellLevel!.upper_price, displayData.token1Symbol)}`
                  : `${formatPrice(displayData.strongestSellWall!.price_lower, displayData.token1Symbol)} - ${formatPrice(displayData.strongestSellWall!.price_upper, displayData.token1Symbol)}`}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
