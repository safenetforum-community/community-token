use ant_networking::{GetRecordError, NetworkError};
use autonomi::{
    client::payment::PaymentOption, scratchpad::ScratchpadError, Bytes, Client, PublicKey,
    ScratchpadAddress, SecretKey, XorName,
};
use futures::Future;
use ruint::aliases::U256;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Wallet(
    PublicKey,
    HashMap<Option<XorName>, (U256, Vec<(PublicKey, U256)>)>,
    U256,
);
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
        println!("request token_id: {:?}", req_token_id);
        let index = self.1.get(&req_token_id).map(|(index, _spends)| index);
        println!("found index: {:?}", index);

        let index = match index {
            Some(index) => index,
            None => {
                self.2 = self
                    .2
                    .checked_add(U256::from(1)) // increment
                    .ok_or("This wallet is full".to_string())?;
                self.1.insert(req_token_id, (self.2, Vec::new()));
                &self.2
            }
        };
        println!("request index: {}", index);

        let request_key = self.0.derive_child(&index.to_be_bytes::<32>());
        println!(
            "request Derived PublicKey: {:.4}(...), {:?}",
            request_key.to_hex(),
            request_key
        );

        Ok(request_key)
    }

    pub fn receive(
        &mut self,
        amount: U256,
        received_token_id: XorName,
        spend: PublicKey,
    ) -> Result<(), String> {
        let entry = self.1.get(&Some(received_token_id));

        match entry {
            None => {
                let none_entry = self.1.remove(&None);
                match none_entry {
                    Some(ne) => {
                        // receiveing tokens, which token_id was not known when requesting.
                        self.1.insert(Some(received_token_id), ne.clone());
                    }
                    None => (),
                }
            }
            Some(_) => (),
        };

        let entry = self.1.get_mut(&Some(received_token_id));

        match entry {
            Some((_index, spends)) => {
                spends.push((spend, amount));
                Some(())
            }
            None => {
                return Err("No requested key in this wallet.".into());
            }
        };

        Ok(())
    }

    pub fn balance_total(&self) -> HashMap<XorName, Result<U256, String>> {
        self.1.iter().fold(
            HashMap::<XorName, Result<U256, String>>::new(),
            |mut token_balances, (token_id, (_index, spends))| {
                if let Some(id) = *token_id {
                    token_balances.insert(
                        id,
                        spends
                            .iter()
                            .fold(Ok(U256::ZERO), |sum_res, (_spend, amount)| {
                                sum_res.and_then(|sum| match sum.overflowing_add(*amount) {
                                    (added, false) => Ok(added),
                                    (_, true) => Err("Overflow.".into()),
                                })
                            }),
                    );
                }

                token_balances
            },
        )
    }

    pub fn balance(&self, token_id: XorName) -> Result<U256, String> {
        let spends = match self.1.get(&Some(token_id)) {
            None => {
                return Ok(U256::ZERO);
            }
            Some((_index, spends)) => spends,
        };

        let (balance, overflow) = spends.iter().fold(
            (U256::ZERO, false),
            |(sum, any_overflow), (_spend, amount)| {
                let (added, this_overflow) = sum.overflowing_add(*amount);
                (added, any_overflow || this_overflow)
            },
        );

        match overflow {
            false => Ok(balance),
            true => Err("Overflow.".into()),
        }
    }

    pub fn take_to_spend(
        &mut self,
        token_id: XorName,
    ) -> Result<(Vec<PublicKey>, U256, PublicKey), String> {
        let (spends, sum, overflow) = self
            .1
            .remove(&Some(token_id))
            .map(|(_index, spends)| {
                let (sum, overflow) = spends.iter().fold(
                    (U256::ZERO, false),
                    |(sum, any_overflow), (spend, amount)| {
                        println!("Input: {:?}", (token_id, spend, amount));
                        let (sum, this_overflow) = sum.overflowing_add(*amount);
                        (sum, any_overflow || this_overflow)
                    },
                );
                (
                    spends.iter().map(|(spend, _amount)| *spend).collect(),
                    sum,
                    overflow,
                )
            })
            .unwrap_or((Vec::new(), U256::ZERO, false));

        match overflow {
            false => self
                .request(Some(token_id))
                .map(|rest_key| (spends, sum, rest_key)),
            true => Err("Overflow.".into()),
        }
    }

    pub fn index_of_token(&self, token_id: XorName) -> Option<U256> {
        self.1.get(&Some(token_id)).map(|(index, _spends)| *index)
    }

    pub fn pk_of_token(&self, token_id: XorName) -> Option<PublicKey> {
        self.index_of_token(token_id)
            .map(|index| self.0.derive_child(&index.to_be_bytes::<32>()))
    }

    pub fn index_that_derives(&self, request: PublicKey) -> Option<U256> {
        self.1
            .iter()
            .filter(|(_, (index, _))| self.0.derive_child(&index.to_be_bytes::<32>()) == request)
            .map(|(_, (index, _))| *index)
            .collect::<Vec<_>>()
            .first()
            .copied()
    }

    pub fn received_spend(&self, token_id: XorName, spend: PublicKey) -> bool {
        self.1
            .get(&Some(token_id))
            .and_then(|(_index, spends)| {
                match spends
                    .iter()
                    .filter(|(received_spend, _amount)| received_spend == &spend)
                    .count()
                {
                    0 => None,
                    _ => Some(()),
                }
            })
            .is_some()
    }
}

pub trait WalletExt {
    fn act_wallet_get(
        &self,
        sk: &SecretKey,
    ) -> impl Future<Output = Result<Option<Wallet>, String>> + Send;

    fn act_wallet_save(
        &mut self,
        wallet: &Wallet,
        sk: &SecretKey,
        payment: &PaymentOption,
    ) -> impl Future<Output = Result<PublicKey, String>> + Send;
}

impl WalletExt for Client {
    async fn act_wallet_get(&self, sk: &SecretKey) -> Result<Option<Wallet>, String> {
        match self.scratchpad_get(&ScratchpadAddress::new(sk.public_key())).await {
			Ok(sp) => {
				let bytes = sp.decrypt_data(sk).map_err(|e| format!("{e}"))?;
				let wallet = rmp_serde::from_slice(&bytes).map_err(|e| format!("{e}"))?;
				Ok(Some(wallet))
			},
			Err(ScratchpadError::Network(NetworkError::GetRecordError(GetRecordError::RecordNotFound))) // workaround for https://github.com/maidsafe/autonomi/pull/2999
					| Err(ScratchpadError::Missing) => Ok(None),
			Err(e) => Err(format!("{e}")),
		}
    }

    async fn act_wallet_save(
        &mut self,
        wallet: &Wallet,
        sk: &SecretKey,
        payment: &PaymentOption,
    ) -> Result<PublicKey, String> {
        println!("saving: {:?}", wallet);
        println!("sk: {:.4}(...)", sk.to_hex());
        let data = rmp_serde::to_vec(&wallet).map_err(|e| format!("{e}"))?;

        let existing = self.act_wallet_get(sk).await?;

        match existing {
            Some(_) => self
                .scratchpad_update(sk, 0, &Bytes::from(data))
                .await
                .map(|_| sk.public_key()),
            None => self
                .scratchpad_create(sk, 0, &Bytes::from(data), payment.clone())
                .await
                .map(|(_paid, address)| *address.owner()),
        }
        .map_err(|e| format!("{e}"))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn de_ser_ialize() -> Result<(), String> {
        use super::*;

        let mut w = Wallet::new(
            SecretKey::from_hex("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b")
                .map_err(|e| format!("{e}"))?
                .public_key(),
        );

        let token_id = XorName::from_content(&[123u8, 45u8]);
        let spend_address = PublicKey::from_hex("a625836b8970244eae677e6338145fe90d9789777a47311d13af4141c16efdeba60d60db86d7972e86e6d17e8b4db0af").map_err(|e| format!("{e}"))?;

        w.request(Some(token_id))?;
        w.receive(U256::from(1), token_id, spend_address)?;

        let data = Bytes::from(rmp_serde::to_vec(&w).map_err(|e| format!("{e}"))?);

        println!("{:x}", data);

        assert_eq!(
			format!("{:x}", data),
			"93dc0030cc876006073c4eccf1cc9d23ccdfcc94cc8fccea3e7ecc9539cc8d6a7b5acccc6a510f24cce2ccd5160bccbbccd9ccf636cca5cc8cccdfcce6cc9ccc8eccba42ccb1cccfccce0fcca60a81dc0020cca4ccfe1bccc8cca631ccbe22ccaecc96ccad524b13ccf64d68ccefccc503cced40cc86ccd6ccaf4ecca906ccc915cce8ccf492c42000000000000000000000000000000000000000000000000000000000000000019192dc0030cca625cc836bcc8970244eccae677e6338145fcce90dcc97cc89777a47311d13ccaf4141ccc16eccfdccebcca60d60ccdbcc86ccd7cc972ecc86cce6ccd17ecc8b4dccb0ccafc4200000000000000000000000000000000000000000000000000000000000000001c4200000000000000000000000000000000000000000000000000000000000000001".to_string()
		);

        let w2 = rmp_serde::from_slice::<Wallet>(&data).map_err(|e| format!("{e}"))?;

        assert_eq!(w, w2);

        Ok(())
    }

    #[test]
    fn received_spend() -> Result<(), String> {
        use super::*;

        let mut w = Wallet::new(SecretKey::random().public_key());
        println!("{w:?}");

        let token_id = XorName::from_content(&[0u8]);
        let spend_address = SecretKey::random().public_key();

        assert_eq!(false, w.received_spend(token_id, spend_address));

        w.request(Some(token_id))?;
        println!("{w:?}");
        w.receive(U256::from(1), token_id, spend_address)?;
        println!("{w:?}");

        assert_eq!(true, w.received_spend(token_id, spend_address));

        Ok(())
    }
}
