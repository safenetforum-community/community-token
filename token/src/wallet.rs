use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use ruint::aliases::U256;
use autonomi::{Client, PublicKey, XorName};



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet(PublicKey, HashMap<Option<XorName>, (U256, Vec<(PublicKey, U256)>)>, U256);
// TODO: ? give index key a name
// TODO: ? optional pubkey ("none" meaning waiting for payment)? this would require supplying amount arg for request.
// TODO: read/write wallet to autonomi: serde.
// TODO: read/write wallet to bytes: serde.

impl Wallet {

	pub fn new(pk: PublicKey) -> Self {
		Self(pk, HashMap::new(), U256::ZERO)
	}

	/// If you're creating a token, `token_id` can be `None`.
	pub fn request(&mut self, req_token_id: Option<XorName>) -> Result<PublicKey, String> {

		let index = self.1.get(&req_token_id).map(|(index, _spends)| index);

		let index = match index {
			Some(index) => index,
			None => {
				self.2 = self.2.checked_add(U256::from(1)) // increment
					.ok_or("This wallet is full".to_string())?;
				self.1.insert(req_token_id, (self.2, Vec::new()));
				&self.2
			},
		};
		println!("request index: {}", index);

		let request_key = self.0.derive_child(&index.to_be_bytes::<32>());
		println!("request Derived PublicKey: {:.4}(...), {:?}", request_key.to_hex(), request_key);

		Ok(request_key)
	}

	pub fn receive(&mut self, amount: U256, received_token_id: XorName, spend: PublicKey, request: &PublicKey) -> Result<(), String> {

		let entry = self.1.get(&Some(received_token_id));

		match entry {
			None => {
				let none_entry = self.1.remove(&None);
				match none_entry {
					Some(ne) => { // receiveing tokens, which token_id was not known when requesting.
						self.1.insert(Some(received_token_id), ne.clone());
					},
					None => (),
				}
			},
			Some(_) => (),
		};

		let entry = self.1.get_mut(&Some(received_token_id));

		match entry {
			Some((index, spends)) => {
				if self.0.derive_child(&index.to_be_bytes::<32>()) == *request {
					spends.push((spend, amount));
					Some(())
				} else {
					return Err("Provided request public key does not belong to this wallet. Try requesting next key for same token.".into());
				}
			},
			None => {
				return Err("No requested key in this wallet.".into());
			},
		};

		// TODO: validate

		Ok(())
	}

	pub fn balance_total(&self) -> HashMap<XorName, Result<U256, String>> {

		self.1.iter()
			.fold(HashMap::<XorName, Result<U256, String>>::new(),
					|mut token_balances, (token_id, (_index, spends))| {

				if let Some(id) = *token_id {
					token_balances.insert(id,
						spends.iter()
							.fold(Ok(U256::ZERO), |sum_res, (_spend, amount)| {

								sum_res.and_then(|sum| {
									match sum.overflowing_add(*amount) {
										(added, false) => Ok(added),
										(_, true) => Err("Overflow.".into()),
									}
								})
							})
					);
				}

				token_balances
			})
	}

	pub fn balance(&self, token_id: XorName) -> Result<U256, String> {

		let spends = match self.1.get(&Some(token_id)) {
			None => {
				return Ok(U256::ZERO);
			},
			Some((_index, spends)) => spends
		};

		let (balance, overflow) = spends.iter()
			.fold((U256::ZERO, false), |(sum, any_overflow), (_spend, amount)| {
				let (added, this_overflow) = sum.overflowing_add(*amount);
				(added, any_overflow || this_overflow)
			});

		match overflow {
			false => Ok(balance),
			true => Err("Overflow.".into()),
		}
	}

	pub fn take_to_spend(&mut self, token_id: XorName) -> Result<(Vec<PublicKey>, U256, PublicKey), String> {

		let (spends, sum, overflow) = self.1.remove(&Some(token_id))
			.map(|(_index, spends)| {
				let (sum, overflow) = spends.iter()
					.fold((U256::ZERO, false), |(sum, any_overflow), (spend, amount)| {
						println!("Input: {:?}", (token_id, spend, amount));
						let (sum, this_overflow) = sum.overflowing_add(*amount);
						(sum, any_overflow || this_overflow)
					});
				(spends.iter()
					.map(|(spend, _amount)| *spend)
					.collect(),

					sum, overflow)
			}).unwrap_or((Vec::new(), U256::ZERO, false));

		match overflow {
			false => {
				self.request(Some(token_id)).map(|rest_key| (spends, sum, rest_key))
			},
			true => Err("Overflow.".into()),
		}
	}

	pub fn find(&self, request: PublicKey) -> Option<U256> {
		self.1.iter()
			.filter(|(_, (index, _))| self.0.derive_child(&index.to_be_bytes::<32>()) == request)
			.map(|(_, (index, _))| *index)
			.collect::<Vec<_>>()
			.first().copied()
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
