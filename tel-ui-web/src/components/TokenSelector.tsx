'use client';

import React, { useState } from 'react';
import { useForm } from 'react-hook-form';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Button } from '@/components/ui/Button';
import { SUPPORTED_CHAINS, SUPPORTED_DEXES, POPULAR_TOKENS } from '@/lib/constants';
import { isValidAddress, shortenAddress } from '@/lib/utils';
import { ChevronDown, Search } from 'lucide-react';

interface TokenSelectorProps {
  onTokensChange: (token0: string, token1: string) => void;
  onFiltersChange: (filters: { chainId?: number; dex?: string }) => void;
}

interface FormData {
  token0: string;
  token1: string;
  chainId: number;
  dex: string;
}

export function TokenSelector({ onTokensChange, onFiltersChange }: TokenSelectorProps) {
  const [showPopular, setShowPopular] = useState(false);
  
  const { register, handleSubmit, setValue, watch, formState: { errors } } = useForm<FormData>({
    defaultValues: {
      token0: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2', // WETH
      token1: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48', // USDC
      chainId: 1,
      dex: '',
    },
  });

  const chainId = watch('chainId');
  const popularTokens = POPULAR_TOKENS[chainId] || [];

  const onSubmit = (data: FormData) => {
    if (!isValidAddress(data.token0) || !isValidAddress(data.token1)) {
      return;
    }
    
    onTokensChange(data.token0, data.token1);
    onFiltersChange({
      chainId: data.chainId,
      dex: data.dex || undefined,
    });
  };

  const handlePopularTokenSelect = (address: string, isToken0: boolean) => {
    setValue(isToken0 ? 'token0' : 'token1', address);
    setShowPopular(false);
  };

  const chainOptions = SUPPORTED_CHAINS.map(chain => ({
    value: chain.id,
    label: chain.name,
  }));

  const dexOptions = [
    { value: '', label: 'All DEXes' },
    ...SUPPORTED_DEXES.filter(dex => dex.enabled).map(dex => ({
      value: dex.name,
      label: dex.displayName,
    })),
  ];

  return (
    <div className="bg-white p-6 rounded-xl shadow-lg border border-gray-200">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-semibold text-gray-900">Select Token Pair</h2>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowPopular(!showPopular)}
          className="text-blue-600"
        >
          Popular Tokens <ChevronDown className="ml-1 h-4 w-4" />
        </Button>
      </div>

      {showPopular && (
        <div className="mb-6 p-4 bg-gray-50 rounded-lg">
          <h3 className="text-sm font-medium text-gray-700 mb-3">Popular Tokens</h3>
          <div className="grid grid-cols-2 gap-2">
            {popularTokens.map((token) => (
              <div key={token.address} className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handlePopularTokenSelect(token.address, true)}
                  className="flex-1 justify-start text-xs"
                >
                  {token.symbol}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handlePopularTokenSelect(token.address, false)}
                  className="flex-1 justify-start text-xs"
                >
                  {token.symbol}
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}

      <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Input
            label="Token 0 Address"
            placeholder="0x..."
            {...register('token0', {
              required: 'Token 0 address is required',
              validate: (value) => isValidAddress(value) || 'Invalid address format',
            })}
            error={errors.token0?.message}
          />
          
          <Input
            label="Token 1 Address"
            placeholder="0x..."
            {...register('token1', {
              required: 'Token 1 address is required',
              validate: (value) => isValidAddress(value) || 'Invalid address format',
            })}
            error={errors.token1?.message}
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Select
            label="Chain"
            options={chainOptions}
            {...register('chainId', { valueAsNumber: true })}
          />
          
          <Select
            label="DEX"
            options={dexOptions}
            {...register('dex')}
          />
        </div>

        <Button type="submit" className="w-full">
          <Search className="mr-2 h-4 w-4" />
          Analyze Liquidity
        </Button>
      </form>
    </div>
  );
} 