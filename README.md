# Uniswap Pool Watcher - Alloy Learning Project

A Rust project focused on learning the [Alloy](https://github.com/alloy-rs/core) library for Ethereum development. This project demonstrates key Alloy features while building a real-time Uniswap V3 pool price monitoring system.

## Learning Objectives

### 1. Alloy Library Fundamentals
- Provider management and HTTP connections
- Smart contract interaction patterns
- Error handling with `eyre` and `Result` types

### 2. Sol! Macro System
- Contract binding generation with `sol!` macro
- Interface definitions for ERC20 tokens
- Function calls and return value handling

### 3. Big Integer Operations
- Working with `U256` and `U512` for precise calculations
- Avoiding floating-point precision issues in DeFi
- Mathematical operations on large numbers

### 4. Uniswap V3 Price Calculation
- Understanding `sqrtPriceX96` format (Q64.96 fixed-point)
- Converting sqrt price to readable token prices
- Handling decimal precision differences between tokens

### 5. Parallel Processing with Tokio
- Async/await patterns for concurrent operations
- Spawning multiple tokio tasks for pool monitoring
- Provider rotation for load balancing and reliability

## Architecture

```
main() → Create Provider Pool → Initialize Pools → Spawn Parallel Tasks → Monitor Prices
   ↓              ↓                    ↓              ↓
HTTP RPC    Pool Contracts      Price Calculation   Continuous Monitoring
```

## Key Components

### Provider Pool
- Multiple HTTP providers for redundancy
- Round-robin provider rotation in tasks
- Load distribution across RPC endpoints

### Pool Management
- Hardcoded Uniswap V3 pool addresses
- Automatic token0/token1 detection
- Fee tier and decimal precision handling

### Price Calculation Engine
- Real-time sqrtPriceX96 parsing
- Mathematical conversion to human-readable prices
- Support for different token decimal configurations

## Supported Pools

Currently monitoring:
- WETH/USDC 0.05% - `0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640`
- WETH/USDC 0.3% - `0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8`

### Installation
```bash
git clone https://github.com/yeguclu/uniswap_pool_watcher.git
cd uniswap_pool_watcher
cargo run
```