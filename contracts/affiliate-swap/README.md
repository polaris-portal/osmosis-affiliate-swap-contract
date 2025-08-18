## Affiliate Swap (CosmWasm on Osmosis)

This contract routes swaps via Osmosis poolmanager and splits the output by an affiliate fee in basis points. The affiliate portion is sent to a configured Osmosis address and the remainder to the swap caller.

### Instantiate

Fields:

- `owner`: admin address
- `affiliate_addr`: osmosis address receiving fees
- `affiliate_bps`: fee in basis points (0-10000)

### Execute

- `ProxySwapWithFee { swap }`
  - Accepts the exact swap payload you would have sent on-chain and proxies it:
    - `SwapExactAmountIn { routes, token_in, token_out_min_amount }`
    - `SplitRouteSwapExactAmountIn { routes, token_in_denom, token_out_min_amount }`
  - The contract overwrites the `sender` internally to the contract address, validates funds, dispatches the swap, and after success splits the token-out amount between affiliate and caller.

Examples:

Regular (single-route):

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

Split-route:

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

Notes:

- Attach funds equal to `token_in` (single) or the sum of `token_in_amount` for the given `token_in_denom` (split).
- `token_out_min_amount` is honored as-is for slippage protection.

### Query

- `Config {}` -> owner, affiliate addr, affiliate bps

### Build and Test

From repository root:

```bash
cargo test -p affiliate-swap
```

Optimize wasm:

```bash
cargo run --bin build-schema -p affiliate-swap
make -C cosmwasm/contracts/affiliate-swap optimize
```

### Deployment (Osmosis testnet)

Use `osmosisd tx wasm store` to upload the optimized wasm, then instantiate with desired params.

### Notes

- Uses stargate messages `MsgSwapExactAmountIn` and `MsgSplitRouteSwapExactAmountIn` under the hood.
- The contract must hold the input funds and forwards the output via bank sends after swap success.
