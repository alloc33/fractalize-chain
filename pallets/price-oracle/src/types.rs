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
}