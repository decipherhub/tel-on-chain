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

const CustomTooltip = ({ active, payload, label }: {
  active?: boolean;
  payload?: Array<{ payload: ChartDataPoint }>;
  label?: string;
}) => {
  if (!active || !payload || !payload.length) return null;

  const data = payload[0].payload;

  return (
    <div className="card p-3 shadow-lg">
      <p className="font-medium text-[var(--foreground)] text-sm mb-2">{label}</p>
      {data.buyLiquidity > 0 && (
        <p className="text-[var(--success)] text-sm">
          Buy: {formatNumber(data.buyLiquidity, { compact: true })}
        </p>
      )}
      {data.sellLiquidity > 0 && (
        <p className="text-[var(--danger)] text-sm">
          Sell: {formatNumber(data.sellLiquidity, { compact: true })}
        </p>
      )}
      {Object.keys(data.dexSources).length > 0 && (
        <div className="mt-2 pt-2 border-t border-[var(--border)]">
          <p className="text-xs text-[var(--foreground-muted)] mb-1">DEX Sources:</p>
          {Object.entries(data.dexSources).map(([dex, amount]) => (
            <p key={dex} className="text-xs text-[var(--foreground-muted)]">
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

  return (
    <g transform={`translate(${x},${y})`}>
      <text x={0} y={0} dy={16} textAnchor="end" fontSize={11} transform="rotate(-45)">
        <tspan fill="var(--foreground)" fontWeight="500">{lines[0]}</tspan>
      </text>
      <text x={0} y={0} dy={28} textAnchor="end" fontSize={10} transform="rotate(-45)">
        <tspan fill="var(--foreground-muted)">{lines[1]}</tspan>
      </text>
    </g>
  );
};

export function LiquidityChart({ data, currentPrice, token0Symbol, token1Symbol, priceType, onPriceTypeChange }: LiquidityChartProps) {
  const [scaleRange, setScaleRange] = useState(10);

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
      <div className="card p-6">
        <div className="flex items-center justify-center h-64 text-[var(--foreground-muted)]">
          No liquidity data available
        </div>
      </div>
    );
  }

  return (
    <div className="card p-6">
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <h2 className="text-lg font-semibold text-[var(--foreground)]">
            Liquidity Distribution
          </h2>
          <p className="text-sm text-[var(--foreground-muted)] mt-1">
            {formatPrice(currentPrice, token1Symbol)} per {token0Symbol}
          </p>
        </div>

        <div className="flex items-center gap-4">
          {/* Price Type Toggle */}
          <div className="flex items-center gap-2 p-1 rounded-lg bg-[var(--surface)]">
            <button
              onClick={() => onPriceTypeChange('wall')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                priceType === 'wall'
                  ? 'bg-[var(--accent)] text-white'
                  : 'text-[var(--foreground-muted)] hover:text-[var(--foreground)]'
              }`}
            >
              Wall Price
            </button>
            <button
              onClick={() => onPriceTypeChange('current')}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                priceType === 'current'
                  ? 'bg-[var(--accent)] text-white'
                  : 'text-[var(--foreground-muted)] hover:text-[var(--foreground)]'
              }`}
            >
              Current
            </button>
          </div>

          {/* Scale Slider */}
          <div className="flex items-center gap-2">
            <span className="text-xs text-[var(--foreground-muted)]">±{scaleRange}%</span>
            <input
              type="range"
              min="5"
              max="50"
              step="5"
              value={scaleRange}
              onChange={(e) => setScaleRange(Number(e.target.value))}
              className="w-20"
            />
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="h-80">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={processedData} margin={{ top: 20, right: 30, left: 20, bottom: 60 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
            <XAxis
              dataKey="percentageLabel"
              height={80}
              interval="preserveStartEnd"
              minTickGap={30}
              tick={<CustomXAxisTick />}
            />
            <YAxis
              tickFormatter={(value) => formatNumber(value, { compact: true })}
              fontSize={11}
              stroke="var(--foreground-muted)"
            />
            <Tooltip content={<CustomTooltip />} />
            <Legend
              wrapperStyle={{ fontSize: '12px', color: 'var(--foreground-muted)' }}
            />
            <Bar dataKey="buyLiquidity" fill="var(--success)" name="Buy" radius={[4, 4, 0, 0]} />
            <Bar dataKey="sellLiquidity" fill="var(--danger)" name="Sell" radius={[4, 4, 0, 0]} />
            <ReferenceLine
              x="0.0%"
              stroke="var(--accent)"
              strokeDasharray="5 5"
              strokeWidth={2}
              label={{ value: "Current", position: "top", fill: "var(--accent)", fontSize: 11 }}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
