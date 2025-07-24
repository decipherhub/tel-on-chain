'use client';

import React from 'react';
import { formatNumber, formatPrice } from '@/lib/utils';
import { LiquidityWall } from '@/types/api';
import { TrendingUp, TrendingDown, DollarSign, Shield } from 'lucide-react';

interface StatsSummaryProps {
  totalBuyLiquidity: number;
  totalSellLiquidity: number;
  strongestBuyWall?: LiquidityWall;
  strongestSellWall?: LiquidityWall;
  currentPrice: number;
  token0Symbol: string;
  token1Symbol: string;
}

export function StatsSummary({
  totalBuyLiquidity,
  totalSellLiquidity,
  strongestBuyWall,
  strongestSellWall,
  currentPrice,
  token0Symbol,
  token1Symbol,
}: StatsSummaryProps) {
  const buyToSellRatio = totalSellLiquidity > 0 ? totalBuyLiquidity / totalSellLiquidity : 0;
  const totalLiquidity = totalBuyLiquidity + totalSellLiquidity;

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
              {formatPrice(currentPrice, token1Symbol)}
            </p>
            <p className="text-xs text-gray-500">{token0Symbol}/{token1Symbol}</p>
          </div>
          
          <div className="text-center">
            <div className="flex items-center justify-center mb-2">
              <Shield className="h-5 w-5 text-purple-600 mr-1" />
              <span className="text-sm font-medium text-gray-700">Total Liquidity</span>
            </div>
            <p className="text-2xl font-bold text-gray-900">
              {formatNumber(totalLiquidity, { compact: true })} {token1Symbol}
            </p>
            <p className="text-xs text-gray-500">Across all DEXes</p>
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
                {formatNumber(totalBuyLiquidity, { compact: true })} {token1Symbol}
              </p>
              <p className="text-sm text-green-700">
                {((totalBuyLiquidity / totalLiquidity) * 100).toFixed(1)}%
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
                {formatNumber(totalSellLiquidity, { compact: true })} {token1Symbol}
              </p>
              <p className="text-sm text-red-700">
                {((totalSellLiquidity / totalLiquidity) * 100).toFixed(1)}%
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
              style={{ width: `${(totalBuyLiquidity / totalLiquidity) * 100}%` }}
            ></div>
          </div>
        </div>
      </div>

      {/* Strongest Walls */}
      <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Strongest Walls</h3>
        
        <div className="space-y-4">
          {strongestBuyWall && (
            <div className="p-3 border border-green-200 rounded-lg">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-green-800">Strongest Buy Wall</span>
                <span className="text-sm font-bold text-green-900">
                  {formatNumber(strongestBuyWall.liquidity_value, { compact: true })} {token1Symbol}
                </span>
              </div>
              <p className="text-sm text-gray-600">
                {formatPrice(strongestBuyWall.price_lower, token1Symbol)} - {formatPrice(strongestBuyWall.price_upper, token1Symbol)}
              </p>
              {Object.keys(strongestBuyWall.dex_sources).length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {Object.entries(strongestBuyWall.dex_sources).map(([dex, amount]) => (
                    <span
                      key={dex}
                      className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded"
                    >
                      {dex}: {formatNumber(amount, { compact: true })}
                    </span>
                  ))}
                </div>
              )}
            </div>
          )}

          {strongestSellWall && (
            <div className="p-3 border border-red-200 rounded-lg">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-red-800">Strongest Sell Wall</span>
                <span className="text-sm font-bold text-red-900">
                  {formatNumber(strongestSellWall.liquidity_value, { compact: true })} {token1Symbol}
                </span>
              </div>
              <p className="text-sm text-gray-600">
                {formatPrice(strongestSellWall.price_lower, token1Symbol)} - {formatPrice(strongestSellWall.price_upper, token1Symbol)}
              </p>
              {Object.keys(strongestSellWall.dex_sources).length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {Object.entries(strongestSellWall.dex_sources).map(([dex, amount]) => (
                    <span
                      key={dex}
                      className="px-2 py-1 bg-red-100 text-red-800 text-xs rounded"
                    >
                      {dex}: {formatNumber(amount, { compact: true })}
                    </span>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
} 