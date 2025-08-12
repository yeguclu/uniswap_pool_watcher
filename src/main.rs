use alloy::{primitives::address, providers::{ProviderBuilder, Provider}, sol};
use std::error::Error;
use alloy::primitives::{utils::format_units, U256};
use std::str::FromStr;
use tokio::time::Duration;
use std::sync::Arc;


const USDC_ADDRESS: alloy::primitives::Address = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const POOL_ADDRESSES: [&str; 2] = [
    "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", // WETH/USDC 0.05%
    "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", // WETH/USDC 0.3%
];

// Generate bindings for the Uniswap V3 pool state interface
sol! {
    #[sol(rpc)]
    contract UniswapV3Pool {

        address public immutable override token0;
        address public immutable override token1;
        uint24 public immutable override fee;

        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
    }
}

sol! {
    #[sol(rpc)]
    interface IERC20 {
        event Approval(address indexed owner, address indexed spender, uint value);
        event Transfer(address indexed from, address indexed to, uint value);
    
        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint);
        function balanceOf(address owner) external view returns (uint);
        function allowance(address owner, address spender) external view returns (uint);
    
        function approve(address spender, uint value) external returns (bool);
        function transfer(address to, uint value) external returns (bool);
        function transferFrom(address from, address to, uint value) external returns (bool);
    }
}

struct Token {
    address: alloy::primitives::Address,
    decimals: u8,
}

struct Pool {
    address: alloy::primitives::Address,
    version: u8, // 1 for v1, 2 for v2
    token0: Token,
    token1: Token,
    fee: alloy::primitives::Uint<24, 1>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Provider (pick your endpoint)
    let mut provider_pool = Vec::new();
    for _ in 0..10 {
        let provider = ProviderBuilder::new()
            .connect("https://reth-ethereum.ithaca.xyz/rpc")
            .await?;
        provider_pool.push(provider);
    }

    let provider = provider_pool[0].clone();
    // Vector to store pool instances
    let mut pools: Vec<Pool> = Vec::new();

    // Create pool instances for each hardcoded address
    for pool_address_str in POOL_ADDRESSES.iter() {
        let pool_address = pool_address_str.parse::<alloy::primitives::Address>()?;
        
        // Initialize pool contract
        let uni = UniswapV3Pool::new(pool_address, &provider);
        
        // Get pool info using multicall
        let multicall = provider.multicall().add(uni.token0()).add(uni.token1()).add(uni.fee());
        let (token0, token1, fee) = multicall.aggregate().await?;
        
        let multicall = provider.multicall()
            .add(IERC20::new(token0, &provider).decimals())
            .add(IERC20::new(token1, &provider).decimals());
        let (token0_decimals, token1_decimals) = multicall.aggregate().await?;

        // Create Pool instance
        let pool = Pool {
            address: pool_address,
            version: 3,
            token0: Token {
                address: token0,
                decimals: token0_decimals,
            },
            token1: Token {
                address: token1,
                decimals: token1_decimals,
            },
            fee
        };

        pools.push(pool);
    }

    let mut handles = Vec::new();
    let pools = Arc::new(pools);
    let provider_pool = Arc::new(provider_pool);

    // 2 pools for now
    for i in 0..2 {
        let pools = Arc::clone(&pools);
        let provider_pool = Arc::clone(&provider_pool);

        let handle =tokio::spawn(async move {
            loop {
                let price = get_price(&pools[i], &provider_pool[i]).await.unwrap();
                println!("price: {price}");
                tokio::time::sleep(Duration::from_secs(12)).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

// very sloppy implementation, havent figured out operatioans and conversions betweeen bigints and floats
async fn get_price(pool: &Pool, provider: &alloy::providers::fillers::FillProvider<alloy::providers::fillers::JoinFill<alloy::providers::Identity, alloy::providers::fillers::JoinFill<alloy::providers::fillers::GasFiller, alloy::providers::fillers::JoinFill<alloy::providers::fillers::BlobGasFiller, alloy::providers::fillers::JoinFill<alloy::providers::fillers::NonceFiller, alloy::providers::fillers::ChainIdFiller>>>>, alloy::providers::RootProvider>) -> Result<f64, Box<dyn Error>> {
    match pool.version {
        3 => {
            // https://medium.com/@jaysojitra1011/uniswap-v3-deep-dive-visualizing-ticks-and-liquidity-provisioning-part-3-081db166243b
            // https://rareskills.io/post/uniswap-v3-sqrtpricex96
            
            // price = (q64_96/2^96)^2 / 10^(token1_decimals - token0_decimals)

            let v3 = UniswapV3Pool::new(pool.address, provider);
            let s0 = v3.slot0().call().await?;
            let q64_96= U256::from(s0.sqrtPriceX96);
            let q64_96 = (q64_96 >> 96) * (q64_96 >> 96);
            let denominator_diff = pool.token1.decimals - pool.token0.decimals;

            let price = format_units(q64_96, denominator_diff).unwrap();
            let mut price = f64::from_str(&price).unwrap();

            if pool.token0.address == USDC_ADDRESS {
                price = 1.0 / price;
            }

            Ok(price)
        }
        _ => {
            panic!("Unsupported pool version");
        }
    }
}
