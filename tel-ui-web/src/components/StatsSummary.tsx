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

  // Use aggregate data if in aggregate mode and data is available
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

  return (
    <div className="space-y-6">
      {/* Overview Stats */}
      <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Market Overview</h3>
        
        <div className="grid grid-cols-2 gap-4">
          <div className="text-center">
            <div className="flex items-center justify-center mb-2">
              <DollarSign className="h-5 w-5 text-blue-600 mr-1" />
              <span className="text-sm font-medium text-gray-700">Current Price</span>
            </div>
            <p className="text-2xl font-bold text-gray-900">
              {formatPrice(displayData.currentPrice, displayData.token1Symbol)}
            </p>
            <p className="text-xs text-gray-500">{displayData.token0Symbol}/{displayData.token1Symbol}</p>
          </div>
          
          <div className="text-center">
            <div className="flex items-center justify-center mb-2">
              <Shield className="h-5 w-5 text-purple-600 mr-1" />
              <span className="text-sm font-medium text-gray-700">Total Liquidity</span>
            </div>
            <p className="text-2xl font-bold text-gray-900">
              {formatNumber(totalLiquidity, { compact: true })} {displayData.token1Symbol}
            </p>
            <p className="text-xs text-gray-500">
              {displayData.isAggregate ? 'Aggregate across pairs' : 'Across all DEXes'}
            </p>
          </div>
        </div>
      </div>

      {/* Liquidity Breakdown */}
      <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Liquidity Breakdown</h3>
        
        <div className="space-y-4">
          <div className="flex items-center justify-between p-3 bg-green-50 rounded-lg">
            <div className="flex items-center">
              <TrendingUp className="h-5 w-5 text-green-600 mr-2" />
              <div>
                <p className="font-medium text-green-900">Buy Liquidity</p>
                <p className="text-sm text-green-700">Support levels</p>
              </div>
            </div>
            <div className="text-right">
              <p className="font-bold text-green-900">
                {formatNumber(displayData.totalBuyLiquidity, { compact: true })} {displayData.token1Symbol}
              </p>
              <p className="text-sm text-green-700">
                {((displayData.totalBuyLiquidity / totalLiquidity) * 100).toFixed(1)}%
              </p>
            </div>
          </div>

          <div className="flex items-center justify-between p-3 bg-red-50 rounded-lg">
            <div className="flex items-center">
              <TrendingDown className="h-5 w-5 text-red-600 mr-2" />
              <div>
                <p className="font-medium text-red-900">Sell Liquidity</p>
                <p className="text-sm text-red-700">Resistance levels</p>
              </div>
            </div>
            <div className="text-right">
              <p className="font-bold text-red-900">
                {formatNumber(displayData.totalSellLiquidity, { compact: true })} {displayData.token1Symbol}
              </p>
              <p className="text-sm text-red-700">
                {((displayData.totalSellLiquidity / totalLiquidity) * 100).toFixed(1)}%
              </p>
            </div>
          </div>
        </div>

        <div className="mt-4 p-3 bg-gray-50 rounded-lg">
          <div className="flex justify-between items-center">
            <span className="text-sm font-medium text-gray-700">Buy/Sell Ratio</span>
            <span className="font-bold text-gray-900">
              {buyToSellRatio.toFixed(2)}:1
            </span>
          </div>
          <div className="mt-2 w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-green-500 h-2 rounded-full"
              style={{ width: `${(displayData.totalBuyLiquidity / totalLiquidity) * 100}%` }}
            ></div>
          </div>
        </div>
      </div>

      {/* Strongest Walls */}
      <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Strongest Walls</h3>
        
        <div className="space-y-4">
          {((displayData.isAggregate && displayData.strongestBuyLevel) || (!displayData.isAggregate && displayData.strongestBuyWall)) && (
            <div className="p-3 border border-green-200 rounded-lg">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-green-800">Strongest Buy Wall</span>
                <span className="text-sm font-bold text-green-900">
                  {displayData.isAggregate ? 
                    formatNumber(displayData.strongestBuyLevel!.token1_liquidity, { compact: true }) :
                    formatNumber(displayData.strongestBuyWall!.liquidity_value, { compact: true })
                  } {displayData.token1Symbol}
                </span>
              </div>
              <p className="text-sm text-gray-600">
                {displayData.isAggregate ? 
                  `${formatPrice(displayData.strongestBuyLevel!.lower_price, displayData.token1Symbol)} - ${formatPrice(displayData.strongestBuyLevel!.upper_price, displayData.token1Symbol)}` :
                  `${formatPrice(displayData.strongestBuyWall!.price_lower, displayData.token1Symbol)} - ${formatPrice(displayData.strongestBuyWall!.price_upper, displayData.token1Symbol)}`
                }
              </p>
              {!displayData.isAggregate && displayData.strongestBuyWall && Object.keys(displayData.strongestBuyWall.dex_sources).length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {Object.entries(displayData.strongestBuyWall.dex_sources).map(([dex, amount]) => (
                    <span
                      key={dex}
                      className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded"
                    >
                      {dex}: {formatNumber(amount, { compact: true })}
                    </span>
                  ))}
                </div>
              )}
              {displayData.isAggregate && (
                <div className="mt-2">
                  <span className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded">
                    Aggregated across all major pairs
                  </span>
                </div>
              )}
            </div>
          )}

          {((displayData.isAggregate && displayData.strongestSellLevel) || (!displayData.isAggregate && displayData.strongestSellWall)) && (
            <div className="p-3 border border-red-200 rounded-lg">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-red-800">Strongest Sell Wall</span>
                <span className="text-sm font-bold text-red-900">
                  {displayData.isAggregate ? 
                    formatNumber(displayData.strongestSellLevel!.token1_liquidity, { compact: true }) :
                    formatNumber(displayData.strongestSellWall!.liquidity_value, { compact: true })
                  } {displayData.token1Symbol}
                </span>
              </div>
              <p className="text-sm text-gray-600">
                {displayData.isAggregate ? 
                  `${formatPrice(displayData.strongestSellLevel!.lower_price, displayData.token1Symbol)} - ${formatPrice(displayData.strongestSellLevel!.upper_price, displayData.token1Symbol)}` :
                  `${formatPrice(displayData.strongestSellWall!.price_lower, displayData.token1Symbol)} - ${formatPrice(displayData.strongestSellWall!.price_upper, displayData.token1Symbol)}`
                }
              </p>
              {!displayData.isAggregate && displayData.strongestSellWall && Object.keys(displayData.strongestSellWall.dex_sources).length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {Object.entries(displayData.strongestSellWall.dex_sources).map(([dex, amount]) => (
                    <span
                      key={dex}
                      className="px-2 py-1 bg-red-100 text-red-800 text-xs rounded"
                    >
                      {dex}: {formatNumber(amount, { compact: true })}
                    </span>
                  ))}
                </div>
              )}
              {displayData.isAggregate && (
                <div className="mt-2">
                  <span className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded">
                    Aggregated across all major pairs
                  </span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
} 