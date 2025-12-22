# Beerus - Starknet Light Client

Quick start with Docker Compose:

1. Create `.env` file based on `.env-example` and fill in the required values:
   - `POSTGRES_USER` - PostgreSQL username
   - `POSTGRES_PASSWORD` - PostgreSQL password
   - `ETH_RPC` - Ethereum RPC endpoint URL
   - `STARKNET_RPC` - Starknet RPC endpoint URL (must support v0.9 API)
   - `GATEWAY_URL` - Starknet Feeder Gateway URL
   - `DATABASE_URL` - PostgreSQL connection string (automatically set in docker-compose)

2. Run `docker-compose up --build`

3. Wait for initialization. You will see `Started state range verification ...`. It will take some time to verify blocks starting from the latest L1 block.

4. When finished, you'll see `Starting the sync from block ...`. At this point, you can use the light client as RPC at `localhost:3030`.

## Documentation

For detailed information about Beerus architecture, including state synchronization and call execution, see the [Architecture Documentation](doc/architecture.md).

## Getting Started

### Running Beerus for the first time

#### Prerequisites

**PostgreSQL Database Required**

Beerus requires a PostgreSQL database (version 12 or higher) to store L1 and L2 state data. You can either:

1. **Use Docker Compose (recommended for quick start)** - Automatically sets up PostgreSQL. See quick start instructions above.
2. **Set up PostgreSQL separately** - Install and configure PostgreSQL, then provide the connection string in your configuration.

The database will be automatically initialized with the required schema on first run. Make sure the database user has permissions to create tables.

#### Using Configuration File

Copy the configuration file from `etc/conf/beerus.toml` and set up all required fields:
- `eth_rpc` - Ethereum RPC endpoint URL
- `starknet_rpc` - Starknet RPC endpoint URL (must support v0.9 API)
- `gateway_url` - Starknet Feeder Gateway URL
- `database_url` - PostgreSQL connection string

Make sure that providers are compatible. Read more about providers [here](#rpc-providers)

Then run:
```bash
cargo run --release -- -c ./path/to/config.toml
```

#### Using Environment Variables

Alternatively, you can configure Beerus using environment variables:
```bash
export ETH_RPC="https://eth-mainnet.public.blastapi.io"
export STARKNET_RPC="https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
export GATEWAY_URL="https://feeder.alpha-mainnet.starknet.io"
export DATABASE_URL="postgresql://user:password@localhost:5432/beerus"
cargo run --release
```

### Configuration

#### Required Fields

| field   | example | description |
| ----------- | ----------- | ----------- |
| `eth_rpc` | `https://eth-mainnet.public.blastapi.io` | Ethereum RPC endpoint URL for L1 state verification |
| `starknet_rpc` | `https://starknet-mainnet.public.blastapi.io/rpc/v0_9` | Starknet RPC service provider URL (must support v0.9 API) |
| `gateway_url` | `https://feeder.alpha-mainnet.starknet.io` | Starknet Feeder Gateway base URL |
| `database_url` | `postgresql://user:password@localhost:5432/beerus` | PostgreSQL database connection string |

#### Optional Fields

| field   | default | description |
| ----------- | ----------- | ----------- |
| `l2_rate_limit` | `10` | L2 RPC requests per second, min = 1, max = 1000 |
| `l1_range_blocks` | `9` | Number of L1 blocks to fetch in a range, min = 1, max = 100000 |
| `poll_secs` | `10` | Seconds between L2 state sync checks, min = 1, max = 3600 |
| `l1_poll_secs` | `600` | Seconds between L1 state verification checks, min = 30, max = 36000 |
| `rpc_addr` | `0.0.0.0:3030` | Local address to listen for RPC requests |

#### Example Configuration File (`beerus.toml`)

```toml
eth_rpc = "https://eth-mainnet.public.blastapi.io"
starknet_rpc = "https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
gateway_url = "https://feeder.alpha-mainnet.starknet.io"
database_url = "postgresql://postgres:postgres@localhost:5432/beerus"
l2_rate_limit = 10
l1_range_blocks = 9
poll_secs = 10
l1_poll_secs = 600
rpc_addr = "127.0.0.1:3030"
```

#### RPC Providers

Beerus relies on:
- **Ethereum RPC provider** - For L1 state verification
- **Starknet RPC service provider** - Must support v0.9 API
- **Starknet Feeder Gateway URL** - For gateway state queries

##### Starknet RPC Endpoint Requirements

Beerus expects the Starknet RPC provider to serve the [v0.9 of the Starknet OpenRPC specs](https://github.com/starkware-libs/starknet-specs).


You can check if the provider is compatible by running this command:
```bash
# This is an example RPC url. Use your RPC provider url to check if the node is compatible.
STARKNET_RPC_URL="https://starknet-mainnet.core.chainstack.com/{YOUR_API_KEY}/rpc/v0_9"
curl --location $STARKNET_RPC_URL \
--header 'Content-Type: application/json' \
--data '{
  "id": 1,
  "jsonrpc": "2.0",
  "method": "starknet_getStorageProof",
  "params": [
    {
      "block_number": 3027730
    },
    [
    ],
    [
    ],
    [
      {
        "contract_address": "0x06445b2f04abaab412ea6881978415bfa4b5b7ee9439ae6e2af9b76c44f8c575",
        "storage_keys": [
          "0x0206f38f7e4f15e87567361213c28f235cccdaa1d7fd34c9db1dfe9489c6a091"
        ]
      }
    ]
  ]
}'
```

If you get a response similar to the one below, then the provider is **not compatible**.
```
{
    "jsonrpc": "2.0",
    "error": {
        "code": 42,
        "message": "the node doesn't support storage proofs for blocks that are too far in the past"
    },
    "id": 1
}
```

We recommend to use chainstack:
- [Chainstack](https://docs.chainstack.com/docs/starknet-tooling)

More API providers can be found [here](https://docs.starknet.io/documentation/tools/api-services/).

## Development

#### Build

```bash
cargo build --release
```

#### Test

```bash
cargo test
```

To generate coverage report use tarpaulin

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

#### Docker

Build the Docker image:
```bash
docker build . -t beerus
```

Run with environment variables:
```bash
docker run \
  -e ETH_RPC="https://eth-mainnet.public.blastapi.io" \
  -e STARKNET_RPC="https://starknet-mainnet.core.chainstack.com/{your_key}/rpc/v0_9" \
  -e GATEWAY_URL="https://feeder.alpha-mainnet.starknet.io" \
  -e DATABASE_URL="postgresql://user:password@host:5432/beerus" \
  -p 3030:3030 \
  -it beerus
```

For production use, prefer Docker Compose (see quick start section above) as it automatically sets up PostgreSQL and handles all configuration.

#### Examples

```bash
ALCHEMY_API_KEY='YOURAPIKEY' cargo run --release --example call
ALCHEMY_API_KEY='YOURAPIKEY' cargo run --release --example state
```

## Security

Beerus follows good practices of security, but 100% security cannot be assured.
Beerus is provided **"as is"** without any **warranty**. Use at your own risk.
