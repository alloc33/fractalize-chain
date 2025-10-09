use crate::protocols::DexProtocol;
use crate::{chains::EvmChain, types::TokenPair};
use sp_core::U256;
use sp_std::vec::Vec;

/// QuickSwap protocol (Polygon chain)
pub struct QuickSwapProtocol;

impl DexProtocol<EvmChain> for QuickSwapProtocol {
	fn get_pool_address(pair: TokenPair) -> &'static str {
		match pair {
			TokenPair::EthUsd => "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d", // ETH/USDC on Polygon
			TokenPair::BtcUsd => "", // WBTC/USDC on Polygon
			TokenPair::SolUsd => "", // SOL/USDC on Polygon
			TokenPair::AvaxUsd => "", // AVAX/USDC on Polygon
		}
	}

	fn get_call_data(_pair: TokenPair) -> Vec<u8> {
		// getReserves() selector: 0x0902f1ac
		sp_std::vec![0x09, 0x02, 0xf1, 0xac]
	}

	fn parse_price(data: Vec<u8>) -> Result<f64, &'static str> {
		if data.len() < 96 {
			return Err("Invalid QuickSwap response length");
		}

		let reserve0_u256 = U256::from_big_endian(&data[0..32]);
		let reserve1_u256 = U256::from_big_endian(&data[32..64]);
		let reserve0_u128 = reserve0_u256.low_u128();
		let reserve1_u128 = reserve1_u256.low_u128();
		
		if reserve0_u128 == 0 || reserve1_u128 == 0 {
			return Err("Zero liquidity in QuickSwap pool");
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
		
		Err("No reasonable ETH price found on QuickSwap")
	}
}
