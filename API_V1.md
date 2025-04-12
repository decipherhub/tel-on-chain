# tel-on-chain API Documentation

This document outlines the API endpoints available in tel-on-chain for accessing aggregated buy/sell wall data from various decentralized exchanges on Ethereum.

## Base URL

```
https://api.tel-on-chain.com/v1
```

## Authentication

All API requests require an API key to be included in the request header:

```
Authorization: Bearer YOUR_API_KEY
```

To obtain an API key, please contact the tel-on-chain team.

## Rate Limiting

- 60 requests per minute per API key

## Endpoints

### Get Real-time Liquidity

Retrieve the current liquidity distribution for a specific token pair or all pairs across multiple DEXs.

```http
GET /liquidity/realtime
```

**Parameters:**

| Parameter  | Type     | Required | Description                                                 |
| ---------- | -------- | -------- | ----------------------------------------------------------- |
| baseToken  | string   | Yes      | Base token address                                          |
| quoteToken | string   | No       | Quote token address (default: "all")                        |
| dexs       | string[] | No       | Array of DEX names to include (default: all supported DEXs) |
| pools      | string[] | No       | Array of specific pool identifiers to query                 |
| resolution | number   | No       | Price range resolution in basis points (default: 100)       |
| timeframe  | string   | No       | Time window for aggregation (default: "24h")                |

**Response:**

For specific quote token:

```json
{
  "timestamp": "2024-03-27T12:00:00Z",
  "blockNumber": "19250000",
  "baseToken": {
    "address": "0x...",
    "symbol": "ETH",
    "decimals": 18
  },
  "quoteToken": {
    "address": "0x...",
    "symbol": "USDC",
    "decimals": 6
  },
  "distributions": [
    {
      "priceLevel": "1950.00",
      "buyLiquidity": "1000000.00",
      "sellLiquidity": "500000.00",
      "sources": [
        {
          "dex": "Uniswap V3",
          "poolId": "eth_usdc_500",
          "buyLiquidity": "600000.00",
          "sellLiquidity": "300000.00"
        }
      ]
    }
  ],
  "keyMetrics": {
    "averageLiquidityDepth": "1000000.00",
    "liquidityConcentration": 0.75,
    "strongestSupportLevel": "1920.00",
    "strongestResistanceLevel": "2050.00"
  }
}
```

For all quote tokens (when quoteToken = "all"):

```json
{
  "timestamp": "2024-03-27T12:00:00Z",
  "blockNumber": "19250000",
  "baseToken": {
    "address": "0x...",
    "symbol": "ETH",
    "decimals": 18
  },
  "aggregatedData": {
    "totalLiquidity": "5000000000.00",
    "liquidityDistribution": {
      "dexs": [
        {
          "name": "Uniswap V3",
          "liquidity": "2500000000.00",
          "percentage": 50.0,
          "pools": [
            {
              "id": "eth_usdc_500",
              "fee": 500,
              "liquidity": "1500000000.00",
              "percentage": 30.0
            },
            {
              "id": "eth_usdc_3000",
              "fee": 3000,
              "liquidity": "1000000000.00",
              "percentage": 20.0
            }
          ]
        },
        {
          "name": "Curve",
          "liquidity": "1500000000.00",
          "percentage": 30.0,
          "pools": [
            {
              "id": "eth_usdc_pool",
              "liquidity": "1500000000.00",
              "percentage": 30.0
            }
          ]
        }
      ]
    },
    "majorPairs": [
      {
        "quoteToken": {
          "symbol": "USDC",
          "address": "0x...",
          "liquidity": "2500000000.00",
          "percentage": 50.0
        }
      }
    ],
    "keyMetrics": {
      "averageLiquidityDepth": "1000000.00",
      "liquidityConcentration": 0.75,
      "strongestSupportLevel": "1920.00",
      "strongestResistanceLevel": "2050.00"
    }
  }
}
```

### Get Historical Liquidity

Retrieve historical liquidity data for a specific time period.

```http
GET /liquidity/historical
```

**Parameters:**

| Parameter  | Type    | Required | Description                   |
| ---------- | ------- | -------- | ----------------------------- |
| baseToken  | string  | Yes      | Base token address            |
| quoteToken | string  | Yes      | Quote token address           |
| startTime  | ISO8601 | Yes      | Start timestamp               |
| endTime    | ISO8601 | Yes      | End timestamp                 |
| interval   | string  | No       | Time interval (default: "1h") |

**Response:**

```json
{
  "data": [
    {
      "timestamp": "2024-03-27T11:00:00Z",
      "blockNumber": "19249900",
      "distributions": [
        {
          "priceLevel": "1950.00",
          "buyLiquidity": "1000000.00",
          "sellLiquidity": "500000.00"
        }
      ]
    }
  ]
}
```

### Get Supported DEXs

Retrieve the list of supported decentralized exchanges on Ethereum.

```http
GET /dexs
```
