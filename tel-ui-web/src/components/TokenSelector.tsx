'use client';

import React, { useState } from 'react';
import { useForm } from 'react-hook-form';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Button } from '@/components/ui/Button';
import { SUPPORTED_CHAINS, SUPPORTED_DEXES, POPULAR_POOLS } from '@/lib/constants';
import { isValidAddress } from '@/lib/utils';
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
      token0: '0xe8f7c89c5efa061e340f2d2f206ec78fd8f7e124',
      token1: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      chainId: 1,
      dex: '',
    },
  });

  const chainId = watch('chainId');
  const popularPools = POPULAR_POOLS[chainId] || [];

  const onSubmit = (data: FormData) => {
    if (!isValidAddress(data.token0) || !isValidAddress(data.token1)) return;
    onTokensChange(data.token0, data.token1);
    onFiltersChange({ chainId: data.chainId, dex: data.dex || undefined });
  };

  const handlePopularPoolSelect = (pool: typeof popularPools[0]) => {
    setValue('token0', pool.token0.address);
    setValue('token1', pool.token1.address);
    setShowPopular(false);
  };

  const chainOptions = SUPPORTED_CHAINS.map(chain => ({ value: chain.id, label: chain.name }));
  const dexOptions = [
    { value: '', label: 'All DEXes' },
    ...SUPPORTED_DEXES.filter(dex => dex.enabled).map(dex => ({
      value: dex.name,
      label: dex.displayName,
    })),
  ];

  return (
    <div className="card p-6">
      <div className="flex items-center justify-between mb-5">
        <h2 className="text-lg font-semibold text-[var(--foreground)]">Select Token Pair</h2>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowPopular(!showPopular)}
          className="text-[var(--accent)]"
        >
          Popular <ChevronDown className={`h-4 w-4 transition-transform ${showPopular ? 'rotate-180' : ''}`} />
        </Button>
      </div>

      {showPopular && (
        <div className="mb-5 p-4 rounded-lg bg-[var(--surface)] animate-fade-in">
          <div className="flex flex-wrap gap-2">
            {popularPools.map((pool, index) => (
              <Button
                key={index}
                variant="outline"
                size="sm"
                onClick={() => handlePopularPoolSelect(pool)}
              >
                {pool.name}
              </Button>
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
              required: 'Required',
              validate: (value) => isValidAddress(value) || 'Invalid address',
            })}
            error={errors.token0?.message}
          />
          <Input
            label="Token 1 Address"
            placeholder="0x..."
            {...register('token1', {
              required: 'Required',
              validate: (value) => isValidAddress(value) || 'Invalid address',
            })}
            error={errors.token1?.message}
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Select label="Chain" options={chainOptions} {...register('chainId', { valueAsNumber: true })} />
          <Select label="DEX" options={dexOptions} {...register('dex')} />
        </div>

        <Button type="submit" className="w-full">
          <Search className="h-4 w-4" />
          Analyze Liquidity
        </Button>
      </form>
    </div>
  );
}
