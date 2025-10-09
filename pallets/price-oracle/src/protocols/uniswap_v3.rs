use super::DexProtocol;
use crate::{chains::EvmChain, types::TokenPair};
use alloy_primitives::{hex, U256};
use sp_std::vec::Vec;

/// Uniswap V3 protocol (works on any EVM chain)
pub struct UniswapV3Protocol;

impl DexProtocol<EvmChain> for UniswapV3Protocol {
	fn get_pool_address(pair: TokenPair) -> &'static str {
		match pair {
			TokenPair::EthUsd => "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8", // ETH/USDC 0.3%
			TokenPair::BtcUsd => "",                                           // WBTC/USDC 0.3%
			TokenPair::SolUsd => "",                                           // SOL/USDC 0.3%
			TokenPair::AvaxUsd => "",                                          // AVAX/USDC 0.3%
		}
	}

	fn get_call_data(_pair: TokenPair) -> Vec<u8> {
		hex!("3850c7bd").to_vec() // slot0() selector
	}

	fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
		if data.len() < 32 {
			return Err("Invalid Uniswap V3 response length");
		}

		// Extract sqrtPriceX96 from slot0() response
		let sqrt_price_x96 = U256::from_be_slice(&data[0..32]);
		let sqrt_price_x96_u128 = sqrt_price_x96.to::<u128>();

		if sqrt_price_x96_u128 == 0 {
			return Err("Invalid sqrtPriceX96");
		}

		// Convert sqrtPriceX96 to actual price
		// price = (sqrtPriceX96 / 2^96)^2 * (10^(decimal1 - decimal0))
		let sqrt_price_f64 = sqrt_price_x96_u128 as f64;
		let q96 = (2_u128.pow(96)) as f64;
		let price_ratio = (sqrt_price_f64 / q96) * (sqrt_price_f64 / q96);

		// USDC has 6 decimals, ETH has 18 decimals
		// So we need to multiply by 10^(6-18) = 10^(-12) = 1e-12
		let price_adjusted = price_ratio * 1e-12;

		// Since this gives us USDC per ETH, we need to invert for ETH per USDC
		let eth_price = 1.0 / price_adjusted;

		if eth_price > 1000.0 && eth_price < 20000.0 {
			Ok(eth_price)
		} else {
			Err("Price out of reasonable range")
		}
	}
}

