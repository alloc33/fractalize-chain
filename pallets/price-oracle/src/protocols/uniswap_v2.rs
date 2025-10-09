use super::DexProtocol;
use crate::{chains::EvmChain, types::TokenPair};
use alloy_primitives::{hex, U256};
use sp_std::vec::Vec;

/// Uniswap V2 protocol (works on any EVM chain - SushiSwap, etc.)
pub struct UniswapV2Protocol;

impl DexProtocol<EvmChain> for UniswapV2Protocol {
	fn get_pool_address(pair: TokenPair) -> &'static str {
		match pair {
			TokenPair::EthUsd => "0x397ff1542f962076d0bfe58ea045ffa2d347aca0",
			TokenPair::BtcUsd => "", // WBTC/USDC
			TokenPair::SolUsd => "",
			TokenPair::AvaxUsd => "",
		}
	}

	fn get_call_data(_pair: TokenPair) -> Vec<u8> {
		hex!("0902f1ac").to_vec() // getReserves() selector
	}

	fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
		if data.len() < 96 {
			return Err("Invalid V2 response length");
		}

		let reserve0_u256 = U256::from_be_slice(&data[0..32]);
		let reserve1_u256 = U256::from_be_slice(&data[32..64]);
		let reserve0_u128 = reserve0_u256.to::<u128>();
		let reserve1_u128 = reserve1_u256.to::<u128>();

		if reserve0_u128 == 0 || reserve1_u128 == 0 {
			return Err("Zero liquidity in pool");
		}

		let reserve0_scaled = reserve0_u128 as f64;
		let reserve1_scaled = reserve1_u128 as f64;
		let ratio1 = reserve0_scaled / reserve1_scaled;
		let ratio2 = reserve1_scaled / reserve0_scaled;

		// Try different decimal adjustments to find reasonable ETH price
		let price_options = [
			ratio1 * 1e12,  // reserve0=USDC, reserve1=ETH
			ratio2 * 1e-12, // reserve0=ETH, reserve1=USDC
			ratio1,         // Same decimals
			ratio2,         // Same decimals inverted
		];

		for price in price_options {
			if price > 1000.0 && price < 20000.0 {
				return Ok(price);
			}
		}

		Err("No reasonable ETH price found")
	}
}

