# tel-on-chain API v1 Documentation

The tel-on-chain API provides access to DEX buy/sell wall data and liquidity analytics across multiple chains and protocols.

## Base URL

```
https://api.tel-on-chain.com/v1
```

For local development:

```
http://localhost:8080/v1
```

## Authentication

**TODO: Implement API authentication**

Authentication will be required for production use. The following methods will be supported:

- API Key: Pass via `x-api-key` header
- JWT Tokens: For authenticated user sessions

During development, authentication is disabled by default.

## Endpoints

### Health Check

```
GET /health
```

Returns `200 OK` if the service is healthy.

### Get Liquidity Walls

```
GET /liquidity/walls/:token0/:token1
```

Returns buy/sell wall data for a token pair across all supported DEXes.

**Path Parameters:**

- `token0`: The address of the first token
- `token1`: The address of the second token

**Query Parameters:**

- `dex`: (Optional) Filter results by DEX name (e.g., "uniswap_v2", "uniswap_v3")
- `chain_id`: (Optional) Filter results by chain ID (e.g., 1 for Ethereum mainnet)

**Example Request:**

```
GET /liquidity/walls/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2/0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48?chain_id=1
```

**Example Response:**

```json
{
  "token0": {
    "address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "symbol": "WETH",
    "name": "Wrapped Ether",
    "decimals": 18,
    "chain_id": 1
  },
  "token1": {
    "address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "symbol": "USDC",
    "name": "USD Coin",
    "decimals": 6,
    "chain_id": 1
  },
  "price": 1625.75,
  "buy_walls": [
    {
      "price_lower": 1550.0,
      "price_upper": 1600.0,
      "liquidity_value": 25000000.0,
      "dex_sources": {
        "uniswap_v3": 15000000.0,
        "uniswap_v2": 10000000.0
      }
    },
    {
      "price_lower": 1500.0,
      "price_upper": 1550.0,
      "liquidity_value": 35000000.0,
      "dex_sources": {
        "uniswap_v3": 20000000.0,
        "uniswap_v2": 15000000.0
      }
    }
  ],
  "sell_walls": [
    {
      "price_lower": 1650.0,
      "price_upper": 1700.0,
      "liquidity_value": 30000000.0,
      "dex_sources": {
        "uniswap_v3": 18000000.0,
        "uniswap_v2": 12000000.0
      }
    },
    {
      "price_lower": 1700.0,
      "price_upper": 1750.0,
      "liquidity_value": 40000000.0,
      "dex_sources": {
        "uniswap_v3": 25000000.0,
        "uniswap_v2": 15000000.0
      }
    }
  ],
  "timestamp": "2023-05-01T12:34:56Z"
}
```

### Get Historical Liquidity Data

```
GET /liquidity/history/:token0/:token1
```

Returns historical liquidity data for a token pair over a specified time range.

**Path Parameters:**

- `token0`: The address of the first token
- `token1`: The address of the second token

**Query Parameters:**

- `dex`: (Optional) Filter results by DEX name (e.g., "uniswap_v2", "uniswap_v3")
- `chain_id`: (Optional) Filter results by chain ID (e.g., 1 for Ethereum mainnet)
- `start_time`: (Optional) Start timestamp in ISO format (default: 24 hours ago)
- `end_time`: (Optional) End timestamp in ISO format (default: current time)
- `interval`: (Optional) Time interval between data points in minutes (default: 60)

**Example Request:**

```
GET /liquidity/history/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2/0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48?chain_id=1&start_time=2023-04-30T12:00:00Z&end_time=2023-05-01T12:00:00Z&interval=120
```

**Example Response:**

```json
{
  "token0": {
    "address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "symbol": "WETH",
    "name": "Wrapped Ether",
    "decimals": 18,
    "chain_id": 1
  },
  "token1": {
    "address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    "symbol": "USDC",
    "name": "USD Coin",
    "decimals": 6,
    "chain_id": 1
  },
  "data_points": [
    {
      "timestamp": "2023-04-30T12:00:00Z",
      "price": 1620.5,
      "total_liquidity_value": 120000000.0,
      "buy_wall_strength": 55000000.0,
      "sell_wall_strength": 65000000.0,
      "strongest_buy_wall": {
        "price_lower": 1550.0,
        "price_upper": 1600.0,
        "liquidity_value": 25000000.0
      },
      "strongest_sell_wall": {
        "price_lower": 1650.0,
        "price_upper": 1700.0,
        "liquidity_value": 30000000.0
      }
    },
    {
      "timestamp": "2023-04-30T14:00:00Z",
      "price": 1618.75,
      "total_liquidity_value": 125000000.0,
      "buy_wall_strength": 60000000.0,
      "sell_wall_strength": 65000000.0,
      "strongest_buy_wall": {
        "price_lower": 1550.0,
        "price_upper": 1600.0,
        "liquidity_value": 28000000.0
      },
      "strongest_sell_wall": {
        "price_lower": 1650.0,
        "price_upper": 1700.0,
        "liquidity_value": 32000000.0
      }
    }
  ],
  "time_period": {
    "start_time": "2023-04-30T12:00:00Z",
    "end_time": "2023-05-01T12:00:00Z",
    "interval_minutes": 120
  }
}
```

### Get Token Information

```
GET /tokens/:chain_id/:address
```

Returns information about a token.

**Path Parameters:**

- `chain_id`: The chain ID (e.g., 1 for Ethereum mainnet)
- `address`: The token address

**Example Request:**

```
GET /tokens/1/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2
```

**Example Response:**

```json
{
  "address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
  "symbol": "WETH",
  "name": "Wrapped Ether",
  "decimals": 18,
  "chain_id": 1
}
```

### Get Pools by DEX

```
GET /pools/:dex/:chain_id
```

Returns pools available for a specific DEX and chain.

**Path Parameters:**

- `dex`: The DEX name (e.g., "uniswap_v2", "uniswap_v3")
- `chain_id`: The chain ID (e.g., 1 for Ethereum mainnet)

**Example Request:**

```
GET /pools/uniswap_v3/1
```

**Example Response:**

```json
[
  "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
  "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8",
  "0x7bea39867e4169dbe237d55c8242a8f2fcdcc387"
]
```

## Error Responses

The API returns standard HTTP status codes to indicate the success or failure of a request.

**Example Error Response:**

```json
{
  "message": "Invalid token address",
  "code": 400
}
```

## Rate Limiting

Currently, there are no rate limits in place. This may change in the future.

## Versioning

The API version is included in the URL path (e.g., `/v1/liquidity/walls`). Major version changes may include breaking changes.

## Support

For API support, please create an issue on our GitHub repository.
