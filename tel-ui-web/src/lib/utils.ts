import { clsx, type ClassValue } from 'clsx';

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs);
}

export function formatNumber(
  value: number,
  options?: {
    decimals?: number;
    compact?: boolean;
    currency?: boolean;
  }
): string {
  const { decimals = 2, compact = false, currency = false } = options || {};

  if (compact) {
    if (value >= 1e9) {
      return `${currency ? '$' : ''}${(value / 1e9).toFixed(decimals)}B`;
    }
    if (value >= 1e6) {
      return `${currency ? '$' : ''}${(value / 1e6).toFixed(decimals)}M`;
    }
    if (value >= 1e3) {
      return `${currency ? '$' : ''}${(value / 1e3).toFixed(decimals)}K`;
    }
  }

  const formatted = new Intl.NumberFormat('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value);

  return currency ? `$${formatted}` : formatted;
}

export function formatPrice(price: number, token1Symbol?: string): string {
  const suffix = token1Symbol ? ` ${token1Symbol}` : '';
  
  if (price >= 1000) {
    return formatNumber(price, { decimals: 2 }) + suffix;
  }
  if (price >= 1) {
    return formatNumber(price, { decimals: 4 }) + suffix;
  }
  return formatNumber(price, { decimals: 6 }) + suffix;
}

export function formatPriceRange(priceLower: number, priceUpper: number, token1Symbol?: string): string {
  const suffix = token1Symbol ? ` ${token1Symbol}` : '';
  return `${formatNumber(priceLower, { decimals: 4 })} - ${formatNumber(priceUpper, { decimals: 4 })}${suffix}`;
}

export function isValidAddress(address: string): boolean {
  return /^0x[a-fA-F0-9]{40}$/.test(address);
}

export function shortenAddress(address: string, chars = 4): string {
  if (!isValidAddress(address)) return address;
  return `${address.slice(0, 2 + chars)}...${address.slice(-chars)}`;
}

export function calculatePriceImpact(
  currentPrice: number,
  targetPrice: number
): number {
  return ((targetPrice - currentPrice) / currentPrice) * 100;
} 