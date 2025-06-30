'use client';

import React from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
  Legend,
} from 'recharts';
import { formatNumber, formatPrice } from '@/lib/utils';

interface ChartDataPoint {
  priceRange: string;
  price: number;
  buyLiquidity: number;
  sellLiquidity: number;
  type: 'buy' | 'sell';
  dexSources: Record<string, number>;
}

interface LiquidityChartProps {
  data: ChartDataPoint[];
  currentPrice: number;
  token0Symbol: string;
  token1Symbol: string;
}

const CustomTooltip = ({ active, payload, label }: any) => {
  if (!active || !payload || !payload.length) return null;

  const data = payload[0].payload;
  
  return (
    <div className="bg-white p-4 border border-gray-200 rounded-lg shadow-lg">
      <p className="font-semibold text-gray-900 mb-2">{label}</p>
      
      {data.buyLiquidity > 0 && (
        <div className="mb-2">
          <p className="text-green-600 font-medium">
            Buy Liquidity: {formatNumber(data.buyLiquidity, { currency: true, compact: true })}
          </p>
        </div>
      )}
      
      {data.sellLiquidity > 0 && (
        <div className="mb-2">
          <p className="text-red-600 font-medium">
            Sell Liquidity: {formatNumber(data.sellLiquidity, { currency: true, compact: true })}
          </p>
        </div>
      )}
      
      {Object.keys(data.dexSources).length > 0 && (
        <div className="mt-3 pt-2 border-t border-gray-100">
          <p className="text-sm font-medium text-gray-700 mb-1">DEX Sources:</p>
          {Object.entries(data.dexSources).map(([dex, amount]) => (
            <p key={dex} className="text-xs text-gray-600">
              {dex}: {formatNumber(amount as number, { currency: true, compact: true })}
            </p>
          ))}
        </div>
      )}
    </div>
  );
};

export function LiquidityChart({ data, currentPrice, token0Symbol, token1Symbol }: LiquidityChartProps) {
  if (!data || data.length === 0) {
    return (
      <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
        <div className="flex items-center justify-center h-64 text-gray-500">
          No liquidity data available
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-gray-900 mb-2">
          Liquidity Distribution: {token0Symbol}/{token1Symbol}
        </h2>
        <p className="text-sm text-gray-600">
          Current Price: {formatPrice(currentPrice)}
        </p>
      </div>

      <div className="h-96">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart
            data={data}
            margin={{
              top: 20,
              right: 30,
              left: 20,
              bottom: 60,
            }}
          >
            <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
            <XAxis
              dataKey="priceRange"
              angle={-45}
              textAnchor="end"
              height={80}
              fontSize={12}
              interval={0}
            />
            <YAxis
              tickFormatter={(value) => formatNumber(value, { compact: true })}
              fontSize={12}
            />
            <Tooltip content={<CustomTooltip />} />
            <Legend />
            
            {/* Reference line for current price */}
            <ReferenceLine
              x={data.find(d => Math.abs(d.price - currentPrice) === Math.min(...data.map(d => Math.abs(d.price - currentPrice))))?.priceRange}
              stroke="#8884d8"
              strokeDasharray="5 5"
              label={{ value: "Current Price", position: "top" }}
            />
            
            <Bar
              dataKey="buyLiquidity"
              fill="#10b981"
              name="Buy Liquidity"
              radius={[2, 2, 0, 0]}
            />
            <Bar
              dataKey="sellLiquidity"
              fill="#ef4444"
              name="Sell Liquidity"
              radius={[2, 2, 0, 0]}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>

      <div className="mt-4 flex justify-center space-x-6 text-sm">
        <div className="flex items-center">
          <div className="w-3 h-3 bg-green-500 rounded mr-2"></div>
          <span className="text-gray-600">Buy Walls (Support)</span>
        </div>
        <div className="flex items-center">
          <div className="w-3 h-3 bg-red-500 rounded mr-2"></div>
          <span className="text-gray-600">Sell Walls (Resistance)</span>
        </div>
      </div>
    </div>
  );
} 