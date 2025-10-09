use sp_io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPair {
	EthUsd,
	BtcUsd,
	SolUsd,
	AvaxUsd,
}

impl TokenPair {
	pub fn as_str(&self) -> &'static str {
		match self {
			TokenPair::EthUsd => "ETH/USD",
			TokenPair::BtcUsd => "BTC/USD",
			TokenPair::SolUsd => "SOL/USD",
			TokenPair::AvaxUsd => "AVAX/USD",
		}
	}

	pub fn to_hash(&self) -> [u8; 32] {
		sp_io::hashing::blake2_256(self.as_str().as_bytes())
	}

	/// Get reasonable price bounds for this token pair (min, max) in micro USD
	pub fn get_price_bounds(&self) -> (u64, u64) {
		match self {
			TokenPair::EthUsd => (1_000_000_000, 20_000_000_000), // $1,000 - $20,000
			TokenPair::BtcUsd => (20_000_000_000, 200_000_000_000), // $20,000 - $200,000
			TokenPair::SolUsd => (10_000_000, 1_000_000_000),     // $10 - $1,000
			TokenPair::AvaxUsd => (5_000_000, 200_000_000),       // $5 - $200
		}
	}
}

