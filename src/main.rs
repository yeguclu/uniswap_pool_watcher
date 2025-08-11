use alloy::{primitives::address, providers::ProviderBuilder, sol};
use std::error::Error;
use alloy::primitives::{utils::format_units, U256};
use std::str::FromStr;

const USDC_ADDRESS: alloy::primitives::Address = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const WETH_ADDRESS: alloy::primitives::Address = address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

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
    let provider = ProviderBuilder::new()
        .connect("https://reth-ethereum.ithaca.xyz/rpc")
        .await?;

    // Pool address (WETH/USDC 0.05% on mainnet)
    let pool = address!("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");

    // initialize pool
    let uni = UniswapV3Pool::new(pool, &provider);
    let token0 = uni.token0().call().await?;
    let token1 = uni.token1().call().await?;
    let fee = uni.fee().call().await?;
    let token0_decimals = IERC20::new(token0, &provider).decimals().call().await?;
    let token1_decimals = IERC20::new(token1, &provider).decimals().call().await?;

    let pool = Pool {
        address: pool,
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

    let price = get_price(&pool, &provider).await?;

    Ok(())
}

async fn get_price(pool: &Pool, provider: &alloy::providers::fillers::FillProvider<alloy::providers::fillers::JoinFill<alloy::providers::Identity, alloy::providers::fillers::JoinFill<alloy::providers::fillers::GasFiller, alloy::providers::fillers::JoinFill<alloy::providers::fillers::BlobGasFiller, alloy::providers::fillers::JoinFill<alloy::providers::fillers::NonceFiller, alloy::providers::fillers::ChainIdFiller>>>>, alloy::providers::RootProvider>) -> Result<(), Box<dyn Error>> {
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
            println!("denominator_diff: {denominator_diff}");

            let price = format_units(q64_96, denominator_diff).unwrap();
            let price = f64::from_str(&price).unwrap();

            if pool.token0.address == USDC_ADDRESS {
                let price = 1.0 / price;
                println!("price: {}", price);
            } else {
                println!("price: {price}");
            }

            Ok(())
        }
        _ => {
            panic!("Unsupported pool version");
        }
    }
}
