use ruint::aliases::U256;
use std::collections::HashMap;
use autonomi::{Client, PublicKey};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInfo {
	symbol: String,
	name: String,
	decimals: u8,
}

pub type Wallet = HashMap<DerivationIndex, Vec<(PublicKey, U256)>>;
// TODO: give index key a name?
// TODO: optional pubkey ("none" meaning waiting for payment)?

#[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
struct DerivationIndex(Vec<u8>);


trait ActExt {
	// TODO: token operations
}

impl ActExt for Client {
	// TODO: token operations implementation
}



#[cfg(test)]
mod tests {
	use sn_curv::elliptic::curves::ECScalar;
	use autonomi::{SecretKey, Wallet as EvmWallet, Chunk, Bytes, GraphEntry, GraphEntryAddress,
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

	fn amount(n: u64, decimals: u8) -> U256 {
		U256::from(n).checked_mul(
			U256::from(10).checked_pow(U256::from(decimals)).expect("U256 Overflow") // TODO: error/result
		).expect("U256 Overflow")
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
		println!("ANT Balance: {}", evm_wallet.balance_of_tokens()
			.await.map_err(|e| format!("{}", e))?);
		println!("Gas Balance: {}", evm_wallet.balance_of_gas_tokens()
			.await.map_err(|e| format!("{}", e))?);

//		let sk = SecretKey::random();
		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			hex::decode(EVM_PK).map_err(|e| format!("{}", e))?[0..32].try_into().unwrap()
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");

		let index = DerivationIndex(vec![1]);
		let issuer_sk = sk.derive_child(&index.0);
		let issuer_key = issuer_sk.public_key();
		println!("Issuer BLS SecretKey: {:.4}(...)", issuer_sk.to_hex());
		println!("Issuer Derived PublicKey: {:.4}(...), {:?}", issuer_key.to_hex(), issuer_key);

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

		let total_supply: U256 = amount(1_000_000, DECIMALS);
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
		assert_eq!(&genesis_owner_pubkey, genesis_address.owner());

		// populate wallet struct

		let mut wallet = Wallet::new();
		match wallet.get_mut(&index) {
			Some(transactions) => {
				transactions.push((*genesis_address.owner(), total_supply));
			},
			None => {
				wallet.insert(index, vec![(*genesis_address.owner(), total_supply)]);
			}
		};

		println!("Wallet: {:?}", wallet);

		// TODO: save wallet to scratchpad

		// check balance on that key

		let tx = client.graph_entry_get(&GraphEntryAddress::new(*genesis_address.owner()))
			.await.map_err(|e| format!("{}", e))?;
		let (balance, overflow) = tx.descendants.iter()
			.filter(|(pubkey, _data)| pubkey == &issuer_key)
			.map(|(_pubkey, data)| U256::from_be_bytes(*data))
			.fold((U256::from(0), false), |(sum, any_overflow), n| {
				let (sum, this_overflow) = sum.overflowing_add(n);
				(sum, any_overflow || this_overflow)
			});
		
		println!("ACT Token issuer Balance: {}", balance);
		assert!(!overflow); // TODO: error/result
		assert_eq!(total_supply, balance);

		// request payment

		let receive_amount = amount(200, DECIMALS);

		let receive_index = DerivationIndex(vec![2]);
		let receiver_sk = sk.derive_child(&receive_index.0);
		let receiver_key = receiver_sk.public_key();
		println!("Receiver BLS SecretKey: {:.4}(...)", receiver_sk.to_hex());
		println!("Receiver Derived PublicKey: {:.4}(...), {:?}", receiver_key.to_hex(), receiver_key);

		if wallet.get(&receive_index).is_none() {
			wallet.insert(receive_index.clone(), vec![]);
		}

		println!("Wallet: {:?}", wallet);

		// prepare rest output

		let rest_index = DerivationIndex(vec![3]);
		let rest_sk = sk.derive_child(&rest_index.0);
		let rest_key = rest_sk.public_key();
		println!("Rest BLS SecretKey: {:.4}(...)", rest_sk.to_hex());
		println!("Rest Derived PublicKey: {:.4}(...), {:?}", rest_key.to_hex(), rest_key);

		if wallet.get(&rest_index).is_none() {
			wallet.insert(rest_index.clone(), vec![]);
		}

		println!("Wallet: {:?}", wallet);

		// spend

		let inputs_amounts = wallet.remove(&DerivationIndex(vec![1])).expect("There should be inputs");
		let (mut inputs, sum, overflow) = inputs_amounts.iter()
			.fold(
				(Vec::<PublicKey>::new(), U256::from(0), false),
				|(mut inputs, sum, any_overflow), (input, amount)| {
					println!("Ipnut: {:?}", (input, amount));
					let (sum, this_overflow) = sum.overflowing_add(*amount);
					inputs.push(*input);
					(inputs, sum, any_overflow || this_overflow)
				}
			);
		assert!(!overflow); // TODO: error/result
		inputs.insert(0, *genesis_address.owner());
		println!("Inputs: {:?}", (&inputs, sum, overflow));

		let rest_amount = sum.checked_sub(receive_amount).unwrap();
		let spend = GraphEntry::new(
			&issuer_sk,
			inputs,
			token_info_address.xorname().clone().0,
			vec![
				(receiver_key, receive_amount.to_be_bytes()),
				(rest_key, rest_amount.to_be_bytes())
			] // all output to issuer
		);
		let (_paid, spend_address) = client.graph_entry_put(spend, with_wallet.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("Spend GraphEntry: {}", spend_address);
		assert_eq!(&issuer_key, spend_address.owner());

		match wallet.get_mut(&rest_index) {
			Some(transactions) => {
				transactions.push((*spend_address.owner(), rest_amount));
			},
			None => {
				wallet.insert(rest_index.clone(), vec![(*spend_address.owner(), rest_amount)]);
			}
		};

		// receive

		match wallet.get_mut(&receive_index) {
			Some(transactions) => {
				transactions.push((*spend_address.owner(), receive_amount));
			},
			None => {
				wallet.insert(receive_index.clone(), vec![(*spend_address.owner(), rest_amount)]);
			}
		};

		println!("Wallet: {:?}", wallet);

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
