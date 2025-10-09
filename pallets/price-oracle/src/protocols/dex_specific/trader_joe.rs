//! # Trader Joe Protocol (Avalanche)
//!
//! Trader Joe V2 implementation for Avalanche network.
//! Special case: fetches AVAX prices and converts to ETH using multiplier.

use crate::{chains::EvmChain, protocols::DexProtocol, types::TokenPair};
use sp_core::U256;
use sp_std::vec::Vec;

/// Trader Joe protocol (Avalanche chain)
pub struct TraderJoeProtocol;

impl DexProtocol<EvmChain> for TraderJoeProtocol {
	fn get_pool_address(pair: TokenPair) -> &'static str {
		// For Trader Joe, we always use AVAX/USDC pool and convert to ETH price
		match pair {
			TokenPair::EthUsd => "0xa389f9430876455c36478deea9769b7ca4e3ddb1",
			TokenPair::BtcUsd => "",
			TokenPair::SolUsd => "",
			TokenPair::AvaxUsd => "",
		}
	}

	fn get_call_data(_pair: TokenPair) -> Vec<u8> {
		// getReserves() selector: 0x0902f1ac
		sp_std::vec![0x09, 0x02, 0xf1, 0xac]
	}

	fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
		if data.len() < 96 {
			return Err("Invalid Trader Joe response length");
		}

		let reserve0_u256 = U256::from_big_endian(&data[0..32]);
		let reserve1_u256 = U256::from_big_endian(&data[32..64]);
		let reserve0_u128 = reserve0_u256.low_u128();
		let reserve1_u128 = reserve1_u256.low_u128();

		if reserve0_u128 == 0 || reserve1_u128 == 0 {
			return Err("Zero liquidity in Trader Joe pool");
		}

		let reserve0_scaled = reserve0_u128 as f64;
		let reserve1_scaled = reserve1_u128 as f64;
		let ratio1 = reserve0_scaled / reserve1_scaled;
		let ratio2 = reserve1_scaled / reserve0_scaled;

		// Try different decimal adjustments to find reasonable AVAX price
		let avax_price_options = [
			ratio1 * 1e12,  // reserve0=USDC, reserve1=AVAX
			ratio2 * 1e-12, // reserve0=AVAX, reserve1=USDC
			ratio1,         // Same decimals
			ratio2,         // Same decimals inverted
		];

		// Find AVAX price in reasonable range ($20-100)
		let avax_price = avax_price_options
			.iter()
			.find(|&&price| price > 20.0 && price < 100.0)
			.copied()
			.ok_or("No reasonable AVAX price found")?;

		// Approximate ETH price from AVAX price (ETH typically ~100-150x AVAX price)
		let eth_price = avax_price * 120.0; // Rough multiplier

		if eth_price > 1000.0 && eth_price < 20000.0 {
			Ok(eth_price)
		} else {
			Err("Trader Joe ETH price out of reasonable range")
		}
	}
}
