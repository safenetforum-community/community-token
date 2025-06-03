use futures::{stream, StreamExt, future::Future};
use serde::{Serialize, Deserialize};
use ruint::aliases::U256;
use autonomi::{SecretKey, Client, PublicKey, client::payment::PaymentOption, XorName, Chunk, Bytes, GraphEntry, GraphEntryAddress, ChunkAddress};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenInfo {
	pub symbol: String,
	pub name: String,
	pub decimals: u8,
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

	fn act_unspent(&self, pubkey: &PublicKey, spend: PublicKey) -> impl Future<Output = Result<(XorName, U256), String>> + Send;

	fn act_balance(&self, pubkey: &PublicKey, spends: Vec<PublicKey>) -> impl Future<Output = Result<U256, String>> + Send;

	fn act_token_info(&self, token_id: &XorName) -> impl Future<Output = Result<TokenInfo, String>> + Send;
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

		let genesis_owner = SecretKey::random();
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

	async fn act_unspent(&self, output: &PublicKey, spend: PublicKey) -> Result<(XorName, U256), String> {

		let tx = self.graph_entry_get(&GraphEntryAddress::new(spend))
			.await.map_err(|e| format!("{}", e))?;

		let (balance, overflow) = tx.descendants.iter()
			.filter(|(pk, _data)| pk == output)
			.map(|(_pk, data)| U256::from_be_bytes(*data))
			.fold((U256::ZERO, false), |(sum, any_overflow), n| {
				let (added, this_overflow) = sum.overflowing_add(n);
				(added, any_overflow || this_overflow)
			});

		match overflow {
			false => Ok((XorName(tx.content), balance)),
			true => Err("Overflow.".into()),
		}
	}

	async fn act_balance(&self, pubkey: &PublicKey, spends: Vec<PublicKey>) -> Result<U256, String> {
		let stream = stream::iter(spends);

		stream.fold(Ok(U256::ZERO), |sum_res, spend_pk| async move {
			let (_token_id, unsp) = self.act_unspent(pubkey, spend_pk).await?;

			match sum_res?.overflowing_add(unsp) {
				(added, false) => Ok(added),
				(_, true) => Err("Overflow.".into()),
			}
		}).await
	}

	async fn act_token_info(&self, token_id: &XorName) -> Result<TokenInfo, String> {
		let token_info_address = ChunkAddress::new(*token_id);

		let chunk = self.chunk_get(&token_info_address)
			.await.map_err(|e| format!("{}", e))?;

		let token_info: TokenInfo = serde_json::from_slice(chunk.value())
			.map_err(|e| format!("{}", e))?;

		Ok(token_info)
	}

//	/// Returns rest amount
//	pub async fn act_spend(from: PublicKey, from_spends: Vec<PublicKey>, amount: U256, to: PublicKey, rest_to: PublicKey) -> Result<U256, String> {
//
//	}
}
