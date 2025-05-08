use std::collections::HashMap;
use autonomi::{PublicKey};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInfo {
	symbol: String,
	name: String,
	decimals: u8,
}

pub type Wallet = HashMap<DerivationIndex, Vec<PublicKey>>;

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
struct DerivationIndex(Vec<u8>);


#[cfg(test)]
mod tests {
	use ruint::aliases::U256;
	use sn_curv::elliptic::curves::ECScalar;
	use autonomi::{Client, SecretKey, Wallet as EvmWallet, Chunk, Bytes, GraphEntry, GraphEntryAddress,
		client::payment::PaymentOption
	};
	use tracing::Level;
	use super::*;

	fn init_logging() {
		let logging_targets = vec![
			("ant_bootstrap".to_string(), Level::INFO),
			("ant_build_info".to_string(), Level::TRACE),
			("ant_evm".to_string(), Level::TRACE),
			("ant_networking".to_string(), Level::INFO),
			("autonomi_cli".to_string(), Level::TRACE),
			("autonomi".to_string(), Level::TRACE),
			("evmlib".to_string(), Level::TRACE),
			("ant_logging".to_string(), Level::TRACE),
			("ant_protocol".to_string(), Level::TRACE),
			("ant_cli".to_string(), Level::TRACE),
		];
		let mut log_builder = ant_logging::LogBuilder::new(logging_targets);
		log_builder.output_dest(ant_logging::LogOutputDest::Stdout);
		log_builder.format(ant_logging::LogFormat::Default);
		let (_, _) = log_builder
			.initialize().expect("Init logging");
	}

	async fn init_alpha() -> Result<Client, String> {
		let client_config = autonomi::ClientConfig {
			init_peers_config: autonomi::InitialPeersConfig {
				network_contacts_url: vec!["http://146.190.225.26/bootstrap_cache.json".to_string()],
				disable_mainnet_contacts: true,
				..Default::default()
			},
			evm_network: autonomi::Network::ArbitrumSepoliaTest,
			strategy: Default::default(),
			network_id: Some(2),
		};
		Client::init_with_config(client_config)
			.await.map_err(|e| format!("{}", e))
	}

	#[tokio::test]
	async fn reads_balance_correctly() -> Result<(), String> {
//		init_logging();
		const EVM_PK: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
		const DECIMALS: u8 = 18;
//		let client = init_alpha().await?;
		let client = Client::init_local()
			.await.map_err(|e| format!("{}", e))?;

		let evm_wallet = EvmWallet::new_from_private_key(client.evm_network().clone(), EVM_PK)
			.map_err(|e| format!("{}", e))?;
		let with_wallet = PaymentOption::from(evm_wallet.clone());
		println!("EVM Address: {}", evm_wallet.address());
		println!("Balance: {}", evm_wallet.balance_of_tokens()
			.await.map_err(|e| format!("{}", e))?);
		println!("Gas Balance: {}", evm_wallet.balance_of_gas_tokens()
			.await.map_err(|e| format!("{}", e))?);

//		let sk = SecretKey::random();
		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			hex::decode(EVM_PK).map_err(|e| format!("{}", e))?[0..32].try_into().unwrap()
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");

		let index = DerivationIndex(vec![1]);
		let issuer_key = sk.derive_child(&index.0).public_key();
		println!("BLS SecretKey: {:.4}(...)", sk.to_hex());
		println!("Derived PublicKey: {:.4}(...), {}", issuer_key.to_hex(), issuer_key);

		// create token info chunk

		let token_info = Chunk::new(Bytes::from(serde_json::to_string(&TokenInfo {
			symbol: "AANT".into(),
			name: "Alternative Autonomi Network Token".into(),
			decimals: DECIMALS,
		}).map_err(|e| format!("{}", e))?));
		let (_paid, token_info_address) = client.chunk_put(&token_info, with_wallet.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("TokenInfo Chunk: {}", token_info_address);

		// create genesis tx with output to given key

		let total_supply: U256 = U256::from(1_000_000).checked_mul(
			U256::from(10).checked_pow(U256::from(DECIMALS)).expect("U256 Overflow") // TODO: error/result
		).expect("U256 Overflow");
		let genesis_owner = SecretKey::random();
		let genesis_owner_pubkey = genesis_owner.public_key();
		let genesis = GraphEntry::new(
			&genesis_owner,
			vec![],
			token_info_address.xorname().clone().0,
			vec![(issuer_key, total_supply.to_be_bytes())] // all output to issuer
		);
		let (_paid, genesis_address) = client.graph_entry_put(genesis, with_wallet.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("Genesis GraphEntry: {}", genesis_address);
		assert_eq!(genesis_owner_pubkey, genesis_address.owner());

		// populate wallet struct

		let mut wallet = Wallet::new();
		match wallet.get_mut(&index) {
			Some(transactions) => {
				transactions.push(*genesis_address.owner());
			},
			None => {
				wallet.insert(index, vec![*genesis_address.owner()]);
			}
		};

		println!("Wallet: {:?}", wallet);
		

		// TODO: check balance on that key

		// TODO: create a spend transaction to a second key

		// TODO: check balance on a second key

		Ok(())
	}

	#[test]
	fn pk_cannot_be_arbitrary_32bytes() -> Result<(), String> {
		// EVM txid
		let pubkey = PublicKey::from_hex("91c680f29bb12c72093642aa6750332e140753bd112097e021428d86b12ee479");
		assert!(pubkey.is_err());

		// EVM address, 20 bytes
		let pubkey = PublicKey::from_hex("a78d8321b20c4ef90ecd72f2588aa985a4bdb684");
		assert!(pubkey.is_err());

		// 32 bytes
		let pubkey = PublicKey::from_hex("a78d8321b20c4ef90ecd72f2588aa985a4bdb684000000000000000000000000");
		assert!(pubkey.is_err());

		Ok(())
	}

	#[test]
	fn can_reference_erc20() -> Result<(), String> {
		// TODO
		assert_eq!(0, 0);

		Ok(())
	}
}
