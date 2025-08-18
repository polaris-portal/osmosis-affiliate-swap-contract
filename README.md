# Osmosis CosmWasm Contracts

This repository contains CosmWasm smart contracts for the Osmosis ecosystem, extracted from the main [Osmosis repository](https://github.com/osmosis-labs/osmosis).

## Contracts

### Affiliate Swap Contract

The affiliate swap contract enables fee collection on swaps through affiliate addresses. This allows partners and integrators to earn fees when users perform swaps through their interfaces.

**Key Features:**

- Affiliate fee collection mechanism
- Configurable fee rates
- Integration with Osmosis swap router
- Support for multiple token pairs

### Swap Router Contract

A general-purpose swap routing contract that can execute multi-hop swaps across different pools on Osmosis.

### Cross-chain Contracts

Several contracts supporting cross-chain functionality:

- **Cross-chain Registry**: Manages cross-chain asset registry
- **Cross-chain Swaps**: Facilitates swaps across different blockchains
- **Outpost**: Manages remote chain interactions

## Development

### Prerequisites

- Rust 1.70+
- `wasm32-unknown-unknown` target
- Docker (for optimization)

### Building

Build all contracts:

```bash
cargo build
```

Build optimized contracts:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.13
```

### Testing

Run tests for all contracts:

```bash
cargo test
```

Run tests for a specific contract:

```bash
cd contracts/affiliate-swap
cargo test
```

## License

This project is licensed under the same terms as the original Osmosis project.

## Contributing

Contributions are welcome! Please follow the standard GitHub pull request workflow.
