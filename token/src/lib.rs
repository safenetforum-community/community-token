use std::fmt;
use ruint::aliases::U256;
use std::collections::HashMap;
use autonomi::{SecretKey, Client, PublicKey, client::payment::PaymentOption, XorName, Chunk, Bytes, GraphEntry};
use serde::{Serialize, Deserialize, Serializer, Deserializer,
	de::{Visitor, MapAccess, value::MapAccessDeserializer}
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInfo {
	symbol: String,
	name: String,
	decimals: u8,
}

#[derive(Clone, Debug)]
pub struct Wallet(SecretKey, HashMap<DerivationIndex, Vec<(PublicKey, U256)>>);
// TODO: ? give index key a name
// TODO: ? optional pubkey ("none" meaning waiting for payment)? this would require supplying amount arg for request.
// TODO: read/write wallet to autonomi: serde.
// TODO: read/write wallet to bytes: serde.

impl Wallet {
	pub fn new(sk: SecretKey) -> Self { // TODO: maybe PublicKey will be sufficient?
		Self(sk, HashMap::new())
	}
	
	pub fn request(&mut self, index: &DerivationIndex) -> PublicKey {
		let receiver_sk = self.0.derive_child(&index.0);
		let receiver_key = receiver_sk.public_key();
		println!("request BLS SecretKey: {:.4}(...)", receiver_sk.to_hex());
		println!("request Derived PublicKey: {:.4}(...), {:?}", receiver_key.to_hex(), receiver_key);

		if self.1.get(index).is_none() {
			self.1.insert(index.clone(), vec![]);
		}

		receiver_key
	}

	pub fn receive(&mut self, amount: U256, spend: PublicKey, index: &DerivationIndex) {
		match self.1.get_mut(index) {
			Some(transactions) => {
				transactions.push((spend, amount));
			},
			None => {
				self.1.insert(index.clone(), vec![(spend, amount)]);
			}
		};
		// TODO: validate
	}

//	pub fn find(&self, request: PublicKey) -> DerivationIndex {
//		// TODO: find index by deriving and comparing with all available indexes
//	}
}

impl Serialize for Wallet {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// do not serialize SecretKey
		self.1.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Wallet {
	fn deserialize<D>(deserializer: D) -> Result<Wallet, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct WalletVisitor;

		impl <'de> Visitor<'de> for WalletVisitor {
			type Value = Wallet;

			fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				write!(f, "a map")
			}

			fn visit_map<A>(self, map: A) -> Result<Wallet, A::Error>
			where A: MapAccess<'de> {
				MapAccessDeserializer::new(map).deserialize_map(self)
			}
		}

		deserializer.deserialize_map(WalletVisitor)
	}
}


pub trait WalletExt {
//	fn act_wallet_get(sk: SecretKey) -> Result<Wallet, String>;
}

impl WalletExt for Client {
//	fn act_wallet_get(sk: SecretKey) -> Result<Wallet, String> {
//		// TODO
//	}
}


#[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct DerivationIndex(Vec<u8>);


pub trait ActExt {
	fn act_create(
		&self,
		name: String,
		symbol: String,
		decimals: u8,
		total_supply: U256,
		issuer_key: PublicKey,
		payment: &PaymentOption,
	) -> impl std::future::Future<Output = Result<(PublicKey, XorName), String>> + Send;
}

impl ActExt for Client {

	async fn act_create(
		&self,
		name: String,
		symbol: String,
		decimals: u8,
		total_supply: U256,
		issuer_key: PublicKey,
		payment: &PaymentOption,
	) -> Result<(PublicKey, XorName), String> {

		// create token info chunk
		let token_info = Chunk::new(Bytes::from(serde_json::to_string(&TokenInfo {
			name,
			symbol,
			decimals,
		}).map_err(|e| format!("{}", e))?));
		let (_paid, token_info_address) = self.chunk_put(&token_info, payment.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("TokenInfo Chunk: {}", token_info_address);

		let token_id = token_info_address.xorname();
		println!("TokenId: {}", token_id);

		// create genesis tx with output to given key

		let genesis_owner = SecretKey::random();
		let genesis_owner_pubkey = genesis_owner.public_key();
		let genesis = GraphEntry::new(
			&genesis_owner,
			vec![],
			token_id.0.clone(),
			vec![(issuer_key, total_supply.to_be_bytes())] // all output to issuer
		);
		let (_paid, genesis_address) = self.graph_entry_put(genesis, payment.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("Genesis GraphEntry: {}", genesis_address);
		let genesis_spend = *genesis_address.owner();

		assert_eq!(genesis_owner_pubkey, genesis_spend);

		Ok((genesis_spend, *token_id))
	}
}



#[cfg(test)]
mod tests {
	use sn_curv::elliptic::curves::ECScalar;
	use autonomi::{Wallet as EvmWallet, GraphEntryAddress};
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


		let mut wallet = Wallet::new(sk.clone());
		let issuer_index = DerivationIndex(vec![1]);
		let issuer_key = wallet.request(&issuer_index);
		let issuer_sk = sk.derive_child(&issuer_index.0);
		println!("Wallet: {:?}", wallet);

		let total_supply: U256 = amount(1_000_000, DECIMALS);
		let (genesis_spend, token_id) = client.act_create(
			"Alternative Autonomi Network Token".into(),
			"AANT".into(),
			DECIMALS,
			total_supply,
			issuer_key,
			&with_wallet,
		).await?;

		// populate wallet struct

		wallet.receive(total_supply, genesis_spend, &issuer_index);
		println!("Wallet: {:?}", wallet);

		// TODO: save wallet to scratchpad

		// check balance on that key

		let tx = client.graph_entry_get(&GraphEntryAddress::new(genesis_spend))
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
		let receive_key = wallet.request(&receive_index);
		println!("Wallet: {:?}", wallet);

		// prepare rest output

		let rest_index = DerivationIndex(vec![3]);
		let rest_key = wallet.request(&rest_index);
		println!("Wallet: {:?}", wallet);

		// spend

		let inputs_amounts = wallet.1.remove(&DerivationIndex(vec![1])).expect("There should be inputs");
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
		inputs.insert(0, genesis_spend);
		println!("Inputs: {:?}", (&inputs, sum, overflow));

		let rest_amount = sum.checked_sub(receive_amount).unwrap();
		let spend = GraphEntry::new(
			&issuer_sk,
			inputs,
			token_id.0,
			vec![
				(receive_key, receive_amount.to_be_bytes()),
				(rest_key, rest_amount.to_be_bytes())
			] // all output to issuer
		);
		let (_paid, spend_address) = client.graph_entry_put(spend, with_wallet.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("Spend GraphEntry: {}", spend_address);
		assert_eq!(&issuer_key, spend_address.owner());

		wallet.receive(rest_amount, *spend_address.owner(), &rest_index);
		wallet.receive(receive_amount, *spend_address.owner(), &receive_index);
		println!("Wallet: {:?}", wallet);

		Ok(())
	}

	// TODO: de/serialze wallet

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
	fn pubkey_from_derived_sk_equals_derived_pubkey() {
		let index = "index1_abcdef";
		println!("index: {:?}", index);
		let sk = SecretKey::random();
		println!("sk: {:?}", sk);
		let pk = sk.public_key();
		println!("pk: {:?}", pk);

		let pubkey_from_derived_sk = sk.derive_child(index.as_bytes()).public_key();
		println!("pubkey_from_derived_sk: {:?}", pubkey_from_derived_sk);
		let derived_pubkey = pk.derive_child(index.as_bytes());
		println!("derived_pubkey: {:?}", derived_pubkey);

		assert_eq!(pubkey_from_derived_sk, derived_pubkey);
	}

}
