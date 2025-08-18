# Osmosis Affiliate Swap Contract

A CosmWasm smart contract for Osmosis that enables affiliate fee collection on swaps. This contract routes swaps via Osmosis poolmanager and splits the output by an affiliate fee in basis points. The affiliate portion is sent to a configured Osmosis address and the remainder to the swap caller.

## Features

- **Affiliate Fee Collection**: Configurable fee rates for partners and integrators
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

The contract overwrites the `sender` internally to the contract address, validates funds, dispatches the swap, and after success splits the token-out amount between affiliate and caller.

### Examples

**Regular (single-route) swap:**

```bash
osmosisd tx wasm execute <CONTRACT_ADDR> '{
  "proxy_swap_with_fee": {
    "swap": {
      "swap_exact_amount_in": {
        "routes": [{"pool_id": 1, "token_out_denom": "uatom"}],
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

- Attach funds equal to `token_in` (single) or the sum of `token_in_amount` for the given `token_in_denom` (split)
- `token_out_min_amount` is honored as-is for slippage protection

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
   osmosisd tx wasm store artifacts/affiliate_swap.wasm --from <key> --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5
   ```

3. Instantiate with desired parameters:
   ```bash
   osmosisd tx wasm instantiate <code-id> '{
     "owner": "osmo1...",
     "affiliate_addr": "osmo1...",
     "affiliate_bps": 100
   }' --from <key> --label "affiliate-swap" --gas-prices 0.025uosmo --gas auto --gas-adjustment 1.5
   ```

## Technical Details

- Uses stargate messages `MsgSwapExactAmountIn` and `MsgSplitRouteSwapExactAmountIn` under the hood
- The contract must hold the input funds and forwards the output via bank sends after swap success
- Extracted from the main [Osmosis repository](https://github.com/osmosis-labs/osmosis) for standalone development

## License

This project is licensed under the same terms as the original Osmosis project.

## Contributing

Contributions are welcome! Please follow the standard GitHub pull request workflow.
