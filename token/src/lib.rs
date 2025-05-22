use std::fmt;
use std::collections::HashMap;
use futures::{stream, StreamExt, future::Future};
use serde::{Serialize, Deserialize, Serializer, Deserializer,
	de::{Visitor, MapAccess, value::MapAccessDeserializer}
};
use ruint::aliases::U256;
use sn_curv::elliptic::curves::ECScalar;
use autonomi::{SecretKey, Client, PublicKey, client::payment::PaymentOption, XorName, Chunk, Bytes, GraphEntry, GraphEntryAddress};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInfo {
	symbol: String,
	name: String,
	decimals: u8,
}

#[derive(Clone, Debug)]
pub struct Wallet(SecretKey, HashMap<Vec<u8>, Vec<(XorName, PublicKey, U256)>>);
// TODO: ? give index key a name
// TODO: ? optional pubkey ("none" meaning waiting for payment)? this would require supplying amount arg for request.
// TODO: read/write wallet to autonomi: serde.
// TODO: read/write wallet to bytes: serde.

impl Wallet {
	pub fn new(sk: SecretKey) -> Self { // TODO: maybe PublicKey will be sufficient?
		Self(sk, HashMap::new())
	}

	pub fn request(&mut self, index: Vec<u8>) -> PublicKey {
		let receiver_sk = self.0.derive_child(&index);
		let receiver_key = receiver_sk.public_key();
		println!("request BLS SecretKey: {:.4}(...)", receiver_sk.to_hex());
		println!("request Derived PublicKey: {:.4}(...), {:?}", receiver_key.to_hex(), receiver_key);

		receiver_key
	}

	pub fn receive(&mut self, amount: U256, token_id: XorName, spend: PublicKey, index: Vec<u8>) {
		match self.1.get_mut(&index) {
			Some(transactions) => {
				transactions.push((token_id, spend, amount));
			},
			None => {
				self.1.insert(index, vec![(token_id, spend, amount)]);
			}
		};
		// TODO: validate
	}

	pub fn balance_total(&self) -> HashMap<XorName, Result<U256, String>> {

		self.1.iter()
			.fold(HashMap::<XorName, Result<U256, String>>::new(),
					|mut token_balances, (_index, spends)| {

				let index_token_balances = spends.iter()
					.fold(HashMap::<XorName, Result<U256, String>>::new(),
							|mut token_balances, (token_id, _spend, amount)| {

						match token_balances.get(token_id) {
							None => token_balances.insert(*token_id, Ok(U256::ZERO)),
							Some(Ok(sum)) => {
								token_balances.insert(*token_id,
									match sum.overflowing_add(*amount) {
										(added, false) => Ok(added),
										(_, true) => Err("Overflow.".into()),
									}
								)
							},
							Some(Err(_)) => None
						};
						token_balances
					});

				for (token_id, sum_res) in index_token_balances.into_iter() {
					match token_balances.get(&token_id) {
						None => token_balances.insert(token_id, sum_res),
						Some(Ok(sum)) => {
							token_balances.insert(token_id,
								sum_res.and_then(|amount| match sum.overflowing_add(amount) {
									(added, false) => Ok(added),
									(_, true) => Err("Overflow.".into()),
								})
							)
						},
						Some(Err(_)) => None
					};
				}

				token_balances
			})
	}

	pub fn balance(&self, token_id: XorName, index: Vec<u8>) -> Result<U256, String> {

		let spends = match self.1.get(&index) {
			None => {
				return Ok(U256::ZERO);
			},
			Some(spends) => spends
		};

		let (balance, overflow) = spends.iter()
			.filter(|(id, _, _)| *id == token_id)
			.fold((U256::ZERO, false), |(sum, any_overflow), (_id, _spend, amount)| {
				let (added, this_overflow) = sum.overflowing_add(*amount);
				(added, any_overflow || this_overflow)
			});

		match overflow {
			false => Ok(balance),
			true => Err("Overflow.".into()),
		}
	}

	pub fn unspent_outputs(&self, token_id: XorName, index: Vec<u8>) -> Result<(Vec<PublicKey>, U256), String> {
		let (outputs, sum, overflow) = self.1.get(&index).unwrap_or(&Vec::new()).iter()
			.filter(|(id, _, _)| *id == token_id)
			.fold(
				(Vec::<PublicKey>::new(), U256::ZERO, false),
				|(mut outputs, sum, any_overflow), (token_id, output, amount)| {
					println!("Input: {:?}", (token_id, output, amount));
					let (sum, this_overflow) = sum.overflowing_add(*amount);
					outputs.push(*output);
					(outputs, sum, any_overflow || this_overflow)
				}
			);

		match overflow {
			false => Ok((outputs, sum)),
			true => Err("Overflow.".into()),
		}
	}


//	pub fn find(&self, request: PublicKey) -> Vec<u8> {
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


pub trait ActExt {
	fn act_create(
		&self,
		name: String,
		symbol: String,
		decimals: u8,
		total_supply: U256,
		issuer_key: PublicKey,
		payment: &PaymentOption,
	) -> impl Future<Output = Result<(PublicKey, XorName), String>> + Send;

	fn unspent(&self, pubkey: &PublicKey, spend: PublicKey) -> impl Future<Output = Result<U256, String>> + Send;

	fn act_balance(&self, pubkey: PublicKey, spends: Vec<PublicKey>) -> impl Future<Output = Result<U256, String>> + Send;
}

impl ActExt for Client {

	async fn act_create(
		&self,
		name: String,
		symbol: String,
		decimals: u8,
		total_supply: U256,
		to: PublicKey,
		payment: &PaymentOption,
	) -> Result<(PublicKey, XorName), String> {

		// create token info chunk
		let token_info_bytes = Bytes::from(serde_json::to_string(&TokenInfo {
			name,
			symbol,
			decimals,
		}).map_err(|e| format!("{}", e))?);

		let token_info = Chunk::new(token_info_bytes.clone());
		let (_paid, token_info_address) = self.chunk_put(&token_info, payment.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("TokenInfo Chunk: {}", token_info_address);

		let token_id = token_info_address.xorname();
		println!("TokenId: {}", token_id);

		// create genesis tx with output to given key

		let genesis_owner = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			&XorName::from_content(&token_info_bytes).0
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");
		println!("Genesis owner: {:?}", genesis_owner);

		let genesis_owner_pubkey = genesis_owner.public_key();
		let genesis = GraphEntry::new(
			&genesis_owner,
			vec![],
			token_id.0.clone(),
			vec![(to, total_supply.to_be_bytes())] // all output to issuer
		);
		let (_paid, genesis_address) = self.graph_entry_put(genesis, payment.clone())
			.await.map_err(|e| format!("{}", e))?;

		println!("Genesis GraphEntry: {}", genesis_address);
		let genesis_spend = *genesis_address.owner();

		assert_eq!(genesis_owner_pubkey, genesis_spend);

		Ok((genesis_spend, *token_id))
	}

	async fn unspent(&self, pubkey: &PublicKey, spend: PublicKey) -> Result<U256, String> {

		let tx = self.graph_entry_get(&GraphEntryAddress::new(spend))
			.await.map_err(|e| format!("{}", e))?;

		let (balance, overflow) = tx.descendants.iter()
			.filter(|(pk, _data)| pk == pubkey)
			.map(|(_pk, data)| U256::from_be_bytes(*data))
			.fold((U256::ZERO, false), |(sum, any_overflow), n| {
				let (added, this_overflow) = sum.overflowing_add(n);
				(added, any_overflow || this_overflow)
			});

		match overflow {
			false => Ok(balance),
			true => Err("Overflow.".into()),
		}
	}

	async fn act_balance(&self, pubkey: PublicKey, spends: Vec<PublicKey>) -> Result<U256, String>
	{
		let stream = stream::iter(spends);

		stream.fold(Ok(U256::ZERO), |sum_res, spend_pk| async move {
			let unsp = self.unspent(&pubkey, spend_pk).await?;

			match sum_res?.overflowing_add(unsp) {
				(added, false) => Ok(added),
				(_, true) => Err("Overflow.".into()),
			}
		}).await
	}

//	async fn act_token_info // TODO

//	/// Returns rest amount
//	pub async fn act_spend(from: PublicKey, from_spends: Vec<PublicKey>, amount: U256, to: PublicKey, rest_to: PublicKey) -> Result<U256, String> {
//
//	}
}



#[cfg(test)]
mod tests {
	use autonomi::{Wallet as EvmWallet};
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
		const EVM_PRIVKEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
		const DECIMALS: u8 = 18;
//		let client = init_alpha().await?;
		let client = Client::init_local()
			.await.map_err(|e| format!("{}", e))?;

		let evm_wallet = EvmWallet::new_from_private_key(client.evm_network().clone(), EVM_PRIVKEY)
			.map_err(|e| format!("{}", e))?;
		let with_wallet = PaymentOption::from(evm_wallet.clone());
		println!("EVM Address: {}", evm_wallet.address());
		println!("ANT Balance: {}", evm_wallet.balance_of_tokens()
			.await.map_err(|e| format!("{}", e))?);
		println!("Gas Balance: {}", evm_wallet.balance_of_gas_tokens()
			.await.map_err(|e| format!("{}", e))?);

//		let sk = SecretKey::random();
		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			hex::decode(EVM_PRIVKEY).map_err(|e| format!("{}", e))?[0..32].try_into().unwrap()
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");

		// TODO: read wallet from scratchpad

		let mut wallet = Wallet::new(sk.clone());
		let issuer_index = vec![1];
		let issuer_key = wallet.request(issuer_index.clone());
		let issuer_sk = sk.derive_child(&issuer_index);
		println!("Wallet: {:?}", wallet);

		let total_supply: U256 = amount(1_000_000, DECIMALS);
		let (genesis_spend, token_id) = client.act_create(
			"Alternative Autonomi Network Token".into(),
			"AANT3".into(),
			DECIMALS,
			total_supply,
			issuer_key,
			&with_wallet,
		).await?;

		// populate wallet struct

		wallet.receive(total_supply, token_id, genesis_spend, issuer_index.clone());
		println!("Wallet: {:?}", wallet);

		// TODO: save wallet to scratchpad

		// check balance on that key

		let balance_validation = client.act_balance(issuer_key, vec![genesis_spend]).await?;
		
		println!("ACT Token issuer Balance: {}", balance_validation);
//		assert!(!overflow); // TODO: error/result
		assert_eq!(total_supply, balance_validation);
//		assert_eq!(total_supply, wallet.balance_total().get(token_id));
		assert_eq!(total_supply, wallet.balance(token_id, issuer_index)?);
//		assert_eq!(total_supply, wallet.balance(wallet.find(issuer_key)));

		// request payment

		let receive_amount = amount(200, DECIMALS);

		let receive_index = vec![2];
		let receive_key = wallet.request(receive_index.clone());
		println!("Wallet: {:?}", wallet);

		// prepare rest output

		let rest_index = vec![3];
		let rest_key = wallet.request(rest_index.clone());
		println!("Wallet: {:?}", wallet);

		// spend

		let (mut inputs, sum) = wallet.unspent_outputs(token_id, vec![1])?;
		wallet.1.remove(&vec![1]).expect("There should be inputs"); // TODO: don't remove other tokens from the index
		inputs.insert(0, genesis_spend);
		println!("Inputs: {:?}", (&inputs, sum));

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

		wallet.receive(receive_amount, token_id, *spend_address.owner(), receive_index);
		wallet.receive(rest_amount, token_id, *spend_address.owner(), rest_index);
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
