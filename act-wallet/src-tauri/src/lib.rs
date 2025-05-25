use std::collections::HashMap;
use futures::{lock::Mutex, stream, StreamExt};
use ruint::aliases::U256;
use sn_curv::elliptic::curves::ECScalar;
use tauri::{State, Manager};
use autonomi::{Client, SecretKey, Wallet,
	client::payment::PaymentOption};
use ant_act::{ActExt, Wallet as ActWallet};


struct AppState {
	client: Client,
	wallet: Wallet,
	sk: SecretKey,
	act_wallet: ActWallet,
}


#[tauri::command]
async fn connect(local: bool, evm_pk: Option<String>, state: State<'_, Mutex<Option<AppState>>>) -> Result<(), String> {
	let mut state = state.lock().await;

	if state.is_none() {
		let client = if local {
			Client::init_local().await
		} else {
			Client::init().await
		}.map_err(|e| format!("{}", e))?;

		let evm_pk = evm_pk.unwrap_or(SecretKey::random().to_hex()); // bls secret key can be used as eth privkey
		let wallet = Wallet::new_from_private_key(client.evm_network().clone(), &evm_pk)
			.map_err(|e| format!("{}", e))?;

		println!("EVM Address: {}", wallet.address());

		let evm_pk = if &evm_pk[0..2] == "0x" {
			&evm_pk[2..]
		} else {
			&evm_pk
		};

		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			hex::decode(evm_pk).map_err(|e| format!("{}", e))?[0..32].try_into().unwrap()
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");

		let act_wallet = ant_act::Wallet::new(sk.clone());

		*state = Some(AppState {
			client,
			wallet,
			sk,
			act_wallet,
		});
	}

	Ok(())
}

#[tauri::command]
async fn create_token(
	name: String,
	symbol: String,
	decimals: u8,
	total_supply: String,
	state: State<'_, Mutex<Option<AppState>>>
) -> Result<String, String> {
	let mut state_opt = state.lock().await;
	let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

	let client = state.client.clone();
	let evm_wallet = state.wallet.clone();
	let act_wallet = &mut state.act_wallet;

	let with_wallet = PaymentOption::from(evm_wallet);

//	let act_wallet = client.act_wallet(sk);
	let owner = act_wallet.request(vec![1]);

	let total_supply = U256::from_str_radix(&total_supply, 10).map_err(|e| format!("{}", e))?;
	let (genesis_spend, token_id) = client.act_create(
		name,
		symbol,
		decimals,
		total_supply,
		owner,
		&with_wallet,
	).await?;

	let received_balance = client.act_balance(&owner, vec![genesis_spend]).await?;

	act_wallet.receive(received_balance, token_id, genesis_spend, vec![1]);

	Ok(genesis_spend.to_hex())
}

#[tauri::command]
async fn balance(state: State<'_, Mutex<Option<AppState>>>) -> Result<(String, String), String> {
	let state_opt = state.lock().await;
	let state = state_opt.as_ref().ok_or("Not connected.")?;

	let wallet = state.wallet.clone();

	let ant = format!("{}", wallet.balance_of_tokens().await.map_err(|e| format!("{}", e))?);
	let eth = format!("{}", wallet.balance_of_gas_tokens().await.map_err(|e| format!("{}", e))?);

	Ok((ant, eth))
}

#[tauri::command]
async fn act_balances(state: State<'_, Mutex<Option<AppState>>>) -> Result<HashMap<String, String>, String> {
	let state_opt = state.lock().await;
	let state = state_opt.as_ref().ok_or("Not connected.")?;
	let wallet = &state.act_wallet;
	let client = &state.client;

	let balances = wallet.balance_total();
	println!("balances: {:?}", balances);
	stream::iter(balances.iter())
		.fold(Ok(HashMap::<String, String>::new()), |balances_res: Result<HashMap<String, String>, HashMap<String, String>>, (token_id, balance_res)| async move {
			let info_res = client.act_token_info(token_id).await;

			let entry_res = match info_res {
				Err(err) => {
					let len = match balances_res {
						Ok(ref b) => b.len(),
						Err(ref b) => b.len(),
					};
					Err(
						(format!("({}) {}", len, err),
						"Token info error".to_string())
					)
				},
				Ok(info) => {
					let balance_res = balance_res.clone()
						.and_then(|balance| {
							U256::from(10).checked_pow(U256::from(info.decimals))
								.map(|divisor| {
									let (intg, rem) = balance.div_rem(divisor);
									let dec = &format!("{}", rem.saturating_add(divisor))[1..]; // remove "1" from beginning
									let dec = dec.trim_end_matches('0'); // remove trailing zeros
									let dec = match dec.len() { 0 => "0", _ => dec };
									format!("{}.{}", intg, dec)
								}).ok_or("Overflow".to_string())
						});

					match balance_res {
						Ok(balance) => Ok((info.symbol, balance)),
						Err(e) => Err((info.symbol, e.to_string())),
					}
				}
			};

			match balances_res {
				Err(mut balances_errors) => {
					let entry = entry_res.unwrap_or_else(|e| e);
					balances_errors.insert(entry.0, entry.1);
					Err(balances_errors)
				},
				Ok(mut balances) => {
					match entry_res {
						Ok(entry) => {
							balances.insert(entry.0, entry.1);
							Ok(balances)
						},
						Err(entry) => {
							balances.insert(entry.0, entry.1);
							Err(balances)
						}
					}
				}
			}
		}).await
		.map_err(|balances_errors: HashMap<String, String>| format!("{:?}", balances_errors))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.invoke_handler(tauri::generate_handler![
			connect,
			create_token,
			balance,
			act_balances,
		])
		.setup(|app| {
			app.manage(Mutex::new(None::<AppState>));
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
