# Osmosis Affiliate Swap Contract

A CosmWasm smart contract for Osmosis that enables affiliate fee collection on swaps. This contract routes swaps via Osmosis poolmanager and collects the affiliate fee from the input in basis points. The affiliate portion of the input is sent upfront to a configured Osmosis address, and the remaining input is used for the swap. The full swap output is then forwarded to the swap caller.

## Features

- **Affiliate Fee Collection**: Configurable fee rates for partners and integrators, deducted from input
- **Swap Routing**: Integration with Osmosis poolmanager for optimal swap execution
- **Multiple Swap Types**: Support for both regular and split-route swaps
- **Slippage Protection**: Honors user-defined minimum output amounts
- **Secure**: Validates funds and uses stargate messages for reliable execution

## Contract Interface

### Instantiate

Fields:

- `owner`: admin address
- `affiliate_addr`: osmosis address receiving fees
- `affiliate_bps`: fee in basis points (0-10000)

### Execute

**`ProxySwapWithFee { swap }`**

Accepts the exact swap payload you would have sent on-chain and proxies it:

- `SwapExactAmountIn { routes, token_in, token_out_min_amount }`
- `SplitRouteSwapExactAmountIn { routes, token_in_denom, token_out_min_amount }`

The contract overwrites the `sender` internally to the contract address, validates funds, deducts the affiliate fee from the input and sends it to the affiliate address, then dispatches the swap with the remaining input. The entire token-out amount is forwarded to the caller.

### Examples

**Regular (single-route) swap:**

```bash
osmosisd tx wasm execute <CONTRACT_ADDR> '{
  "proxy_swap_with_fee": {
    "swap": {
      "swap_exact_amount_in": {
        "routes": [{"pool_id": "1", "token_out_denom": "uatom"}],
        "token_in": {"denom": "uosmo", "amount": "1000000"},
        "token_out_min_amount": "990000"
      }
    }
  }
}' --from <user> --amount 1000000uosmo --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5
```

**Split-route swap:**

```bash
osmosisd tx wasm execute <CONTRACT_ADDR> '{
  "proxy_swap_with_fee": {
    "swap": {
      "split_route_swap_exact_amount_in": {
        "routes": [
          {
            "token_in_amount": "600000",
            "pools": [{"pool_id": 1, "token_out_denom": "uatom"}]
          },
          {
            "token_in_amount": "400000",
            "pools": [{"pool_id": 151, "token_out_denom": "uatom"}]
          }
        ],
        "token_in_denom": "uosmo",
        "token_out_min_amount": "990000"
      }
    }
  }
}' --from <user> --amount 1000000uosmo --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5
```

**Notes:**

- **Attach funds equal to the full original input**: `token_in` (single) or the sum of `token_in_amount` for `token_in_denom` (split). The contract will send the affiliate cut upfront and swap the remainder.
- **Minimum affiliate fee rounding**: If `affiliate_bps > 0` and the computed fee on input would round down to zero for a non-zero input, the contract charges a minimum of 1 unit of the input denom. If this minimum fee fully consumes the input, the swap is skipped and only the affiliate transfer occurs.
- `token_out_min_amount` is honored on the remaining input as-is for slippage protection.

### Query

**`Config {}`** → Returns owner, affiliate addr, affiliate bps

## Development

### Prerequisites

- Rust 1.65+
- `wasm32-unknown-unknown` target
- Docker (for optimization)

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Optimize for Production

Recommended (Docker):

```bash
# Intel/x86_64 hosts
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.17.0

# Apple Silicon (M1/M2)
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer-arm64:0.17.0
```

Optional (Cargo aliases):

```bash
cargo install cargo-run-script

# Intel/x86_64
cargo optimize

# Apple Silicon
cargo optimize-m1
```

### Schema Generation

```bash
cargo run --bin build-schema
```

## Deployment

1. Build optimized wasm (see Optimize for Production above). The artifact will be at `artifacts/affiliate_swap.wasm`.

2. Upload to Osmosis:

   ```bash
   osmosisd tx wasm store artifacts/affiliate_swap.wasm --from faucet --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5 --keyring-backend test --chain-id osmo-test-5 --node https://rpc.testnet.osmosis.zone/
   ```

   https://www.mintscan.io/osmosis-testnet/tx/F1DD7B7D205F1898D088335630C029DAD2FD01C351E747F13648A5D2AC7F20C8?height=34460751

3. Instantiate with desired parameters:

   ```bash
   osmosisd tx wasm instantiate 12860 '{
     "owner": "osmo1nyphwl8p5yx6fxzevjwqunsfqpcxukmtk8t60m",
     "affiliate_addr": "osmo1f94g5n029cl0ffd72k23fjr3vdepd9lse7agxn",
     "affiliate_bps": 100
   }' --admin osmo1nyphwl8p5yx6fxzevjwqunsfqpcxukmtk8t60m --from faucet --label "affiliate-swap" --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5 --keyring-backend test --chain-id osmo-test-5 --node https://rpc.testnet.osmosis.zone/ --yes
   ```

   https://www.mintscan.io/osmosis-testnet/tx/BB90B0A473231A68B2A27C71D89189D0752162319C97398756CB4DCD182ECF12?height=34461134

4. Execute a swap through the contract:

Exact amount in:

```bash
osmosisd tx wasm execute "osmo15xtkacfkn79jcaqdrfxxqs5fsuae7ph0k2yqzlvqtrzqcxqtrujs348ln5" '{
  "proxy_swap_with_fee": {
    "swap": {
      "swap_exact_amount_in": {
        "routes": [{"pool_id": "1", "token_out_denom": "uion"}],
        "token_in": {"denom": "uosmo", "amount": "1000000"},
        "token_out_min_amount": "1"
      }
    }
  }
}' --from faucet --amount 1000000uosmo --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5 --keyring-backend test --chain-id osmo-test-5 --node https://rpc.testnet.osmosis.zone/ --yes
```

https://www.mintscan.io/osmosis-testnet/tx/AEE109DCAF99D318F9CDC40D72734E75D843CC05B1A21123E9928A0F82ABCF28?height=34461805

Split route swap:

```bash
osmosisd tx wasm execute "osmo15xtkacfkn79jcaqdrfxxqs5fsuae7ph0k2yqzlvqtrzqcxqtrujs348ln5" '{
  "proxy_swap_with_fee": {
    "swap": {
      "split_route_swap_exact_amount_in": {
        "routes": [
          {
            "token_in_amount": "100000",
            "pools": [
              {"pool_id": "1", "token_out_denom": "uion"}
            ]
          },
          {
            "token_in_amount": "100000",
            "pools": [
              {"pool_id": "939", "token_out_denom": "uion"}
            ]
          }
        ],
        "token_in_denom": "uosmo",
        "token_out_min_amount": "1"
      }
    }
  }
}' --from faucet --amount 200000uosmo --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5 --keyring-backend test --chain-id osmo-test-5 --node https://rpc.testnet.osmosis.zone/ --yes
```

https://www.mintscan.io/osmosis-testnet/tx/C8AE3FBD7C44DCED157B8412F0C00EC3C8E6F1E180C755C30019A08BD16B3B4C?sector=logs

## License

This project is licensed under the same terms as the original Osmosis project.
