'use client';

import React, { useState, useMemo } from 'react';
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
  percentageFromCurrent?: number;
}

interface LiquidityChartProps {
  data: ChartDataPoint[];
  currentPrice: number;
  token0Symbol: string;
  token1Symbol: string;
  priceType: 'wall' | 'current';
  onPriceTypeChange: (type: 'wall' | 'current') => void;
}

interface TooltipProps {
  active?: boolean;
  payload?: Array<{
    payload: ChartDataPoint;
  }>;
  label?: string;
}

const CustomTooltip = ({ active, payload, label }: TooltipProps) => {
  if (!active || !payload || !payload.length) return null;

  const data = payload[0].payload;
  
  return (
    <div className="bg-white p-4 border border-gray-200 rounded-lg shadow-lg">
      <p className="font-semibold text-gray-900 mb-2">{label}</p>
      
      {data.buyLiquidity > 0 && (
        <div className="mb-2">
          <p className="text-green-600 font-medium">
            Buy Liquidity: {formatNumber(data.buyLiquidity, { compact: true })}
          </p>
        </div>
      )}
      
      {data.sellLiquidity > 0 && (
        <div className="mb-2">
          <p className="text-red-600 font-medium">
            Sell Liquidity: {formatNumber(data.sellLiquidity, { compact: true })}
          </p>
        </div>
      )}
      
      {Object.keys(data.dexSources).length > 0 && (
        <div className="mt-3 pt-2 border-t border-gray-100">
          <p className="text-sm font-medium text-gray-700 mb-1">DEX Sources:</p>
          {Object.entries(data.dexSources).map(([dex, amount]) => (
            <p key={dex} className="text-xs text-gray-600">
              {dex}: {formatNumber(amount as number, { compact: true })}
            </p>
          ))}
        </div>
      )}
    </div>
  );
};

const CustomXAxisTick = ({ x, y, payload }: { x?: number; y?: number; payload?: { value: string } }) => {
  if (!payload?.value || x === undefined || y === undefined) return null;
  
  const lines = payload.value.split('\n');
  const percentage = lines[0];
  const price = lines[1];
  
  return (
    <g transform={`translate(${x},${y})`}>
      <text x={0} y={0} dy={16} textAnchor="end" fontSize={12} transform="rotate(-45)">
        <tspan fill="#374151" fontWeight="bold">{percentage}</tspan>
      </text>
      <text x={0} y={0} dy={30} textAnchor="end" fontSize={11} transform="rotate(-45)">
        <tspan fill="#6b7280">{price}</tspan>
      </text>
    </g>
  );
};

export function LiquidityChart({ data, currentPrice, token0Symbol, token1Symbol, priceType, onPriceTypeChange }: LiquidityChartProps) {
  const [scaleRange, setScaleRange] = useState(10); // Default 10% range

  // Generate percentage intervals based on scale
  const processedData = useMemo(() => {
    if (!data || data.length === 0) return [];
    
    return data.map((point) => {
      const percentageFromCurrent = ((point.price - currentPrice) / currentPrice) * 100;
      return {
        ...point,
        percentageFromCurrent,
        percentageLabel: `${percentageFromCurrent >= 0 ? '+' : ''}${percentageFromCurrent.toFixed(1)}%\n${formatPrice(point.price, token1Symbol)}`
      };
    }).filter(point => Math.abs(point.percentageFromCurrent) <= scaleRange)
    .sort((a, b) => a.percentageFromCurrent - b.percentageFromCurrent);
  }, [data, currentPrice, scaleRange, token1Symbol]);

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
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-xl font-semibold text-gray-900 mb-2">
              Liquidity Distribution: {token0Symbol}/{token1Symbol}
            </h2>
            <p className="text-sm text-gray-600">
              Current Price: {formatPrice(currentPrice, token1Symbol)} per {token0Symbol}
            </p>
          </div>
          
          <div className="flex items-center space-x-6">
            {/* Price Type Toggle */}
            <div className="flex items-center space-x-3">
              <label className="text-sm font-medium text-gray-700">
                Sell Wall Price:
              </label>
              <div className="flex items-center space-x-2">
                <button
                  onClick={() => onPriceTypeChange('wall')}
                  className={`px-3 py-1 text-xs font-medium rounded ${
                    priceType === 'wall'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
                  }`}
                >
                  Wall Price
                </button>
                <button
                  onClick={() => onPriceTypeChange('current')}
                  className={`px-3 py-1 text-xs font-medium rounded ${
                    priceType === 'current'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
                  }`}
                >
                  Current Price
                </button>
              </div>
            </div>

            {/* Scale Control Slider */}
            <div className="flex items-center space-x-3">
              <label htmlFor="scale-range" className="text-sm font-medium text-gray-700">
                Scale:
              </label>
              <div className="flex items-center space-x-2">
                <span className="text-xs text-gray-500">±{scaleRange}%</span>
                <input
                  id="scale-range"
                  type="range"
                  min="5"
                  max="50"
                  step="5"
                  value={scaleRange}
                  onChange={(e) => setScaleRange(Number(e.target.value))}
                  className="w-20 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer slider"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="h-96">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart
            data={processedData}
            margin={{
              top: 20,
              right: 30,
              left: 20,
              bottom: 60,
            }}
          >
            <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
            <XAxis
              dataKey="percentageLabel"
              height={80}
              interval="preserveStartEnd"
              minTickGap={30}
              tick={<CustomXAxisTick />}
            />
            <YAxis
              tickFormatter={(value) => formatNumber(value, { compact: true })}
              fontSize={12}
            />
            <Tooltip content={<CustomTooltip />} />
            <Legend />
            
            {/* Reference line for current price */}
            <ReferenceLine
              x="0.0%"
              stroke="#8884d8"
              strokeDasharray="5 5"
              label={{ value: "Current Price", position: "top" }}
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