mod act;
mod wallet;

pub use wallet::Wallet;
pub use act::{ActExt, TokenInfo};


#[cfg(test)]
mod tests {
//	use sn_curv::elliptic::curves::ECScalar;
	use tracing::Level;
	use autonomi::{Wallet as EvmWallet};

	use ruint::aliases::U256;
	use autonomi::{SecretKey, Client, PublicKey, client::payment::PaymentOption, GraphEntry};


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
//		let client = Client::init_alpha()
		let client = Client::init_local().await
			.map_err(|e| format!("{}", e))?;

		let evm_wallet = EvmWallet::new_from_private_key(client.evm_network().clone(), EVM_PRIVKEY)
			.map_err(|e| format!("{}", e))?;
		let with_wallet = PaymentOption::from(evm_wallet.clone());
		println!("EVM Address: {}", evm_wallet.address());
		println!("ANT Balance: {}", evm_wallet.balance_of_tokens().await
			.map_err(|e| format!("{}", e))?);
		println!("Gas Balance: {}", evm_wallet.balance_of_gas_tokens().await
			.map_err(|e| format!("{}", e))?);

		let sk1 = SecretKey::random();
		let sk2 = SecretKey::random();
//		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
//			hex::decode(EVM_PRIVKEY).map_err(|e| format!("{}", e))?[0..32].try_into().unwrap()
//		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");


		// TODO: read wallet from scratchpad

		let mut wallet1 = Wallet::new(sk1.public_key());
		let issuer_key = wallet1.request(None)?;
		println!("Wallet1: {:?}", wallet1);

		let symbol = "EACT";
		let total_supply: U256 = amount(1_000_000, DECIMALS);
		let (genesis_spend, token_id) = client.act_create(
			"Example Autonomi Community Token".into(),
			symbol.into(),
			DECIMALS,
			total_supply,
			issuer_key,
			&with_wallet,
		).await?;

		// populate wallet struct

		wallet1.receive(total_supply, token_id, genesis_spend, &issuer_key)?;
		println!("Wallet1: {:?}", wallet1);

		// TODO: save wallet to scratchpad

		// check balance on that key

		let balance_validation = client.act_balance(&issuer_key, vec![genesis_spend]).await?;
		println!("ACT Token issuer Balance: {}", balance_validation);
		assert_eq!(total_supply, balance_validation);
		assert_eq!(total_supply, wallet1.balance(token_id)?);

		let balance_total = wallet1.balance_total();
		println!("Wallet1 balance_total: {:?}", balance_total);
		assert_eq!(1, balance_total.len());
		assert_eq!(total_supply, balance_total.get(&token_id).expect("Wrong token_id").clone()?);

		let token_info = client.act_token_info(&token_id).await?;
		println!("ACT Token symbol: {}", token_info.symbol);
		assert_eq!(symbol, token_info.symbol);

		// request payment

		let mut wallet2 = Wallet::new(sk2.public_key());
		let receive_key = wallet2.request(Some(token_id))?;
		println!("Wallet2: {:?}", wallet2);

		// spend

		let receive_amount = amount(200, DECIMALS);

		let issuer_sk = sk1.derive_child(
			&wallet1.find(issuer_key)
				.ok_or("Key not found".to_string())?
				.to_be_bytes::<32>()
		);

		let (inputs, sum, rest_key) = wallet1.take_to_spend(token_id)?;
		println!("Inputs: {:?}", (&inputs, sum));
		let rest_amount = sum.checked_sub(receive_amount)
			.ok_or("Overflow".to_string())?;

		let spend = GraphEntry::new(
			&issuer_sk,
			inputs,
			token_id.0,
			vec![
				(receive_key, receive_amount.to_be_bytes()),
				(rest_key, rest_amount.to_be_bytes())
			] // all output to issuer
		);
		let (_paid, spend_address) = client.graph_entry_put(spend, with_wallet.clone()).await
			.map_err(|e| format!("{}", e))?;

		println!("Spend GraphEntry: {}", spend_address);
		assert_eq!(&issuer_key, spend_address.owner());

		wallet1.receive(rest_amount, token_id, *spend_address.owner(), &rest_key)?;
		println!("Wallet1: {:?}", wallet1);
		wallet2.receive(receive_amount, token_id, *spend_address.owner(), &receive_key)?;
		println!("Wallet2: {:?}", wallet2);

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
