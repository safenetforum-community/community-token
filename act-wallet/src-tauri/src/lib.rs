use std::collections::HashMap;
use futures::{lock::Mutex, stream, StreamExt};
use ruint::aliases::U256;
use sn_curv::elliptic::curves::ECScalar;
use tauri::{State, Manager, Theme};
use autonomi::{Client, SecretKey, Wallet, XorName,
	client::payment::PaymentOption,
};
use ant_act::{ActExt, Wallet as ActWallet, TokenInfo};


struct AppState {
	client: Client,
	wallet: Wallet,
	sk: SecretKey,
	act_wallet: ActWallet,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Network {Local, Alpha,	Main}

#[tauri::command]
async fn connect(network: Network, evm_pk: Option<String>, state: State<'_, Mutex<Option<AppState>>>) -> Result<String, String> {
	let mut state = state.lock().await;

	if state.is_none() {
		let client = match network {
			Network::Local => Client::init_local().await,
			Network::Main => Client::init().await,
			Network::Alpha => Client::init_alpha().await,
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

		let act_wallet = ActWallet::new(sk.public_key());

		*state = Some(AppState {
			client,
			wallet,
			sk,
			act_wallet,
		});

		Ok(evm_pk.to_string())

	} else {
		Err("Already connected.".to_string())
	}
}

#[tauri::command]
async fn is_connected(state: State<'_, Mutex<Option<AppState>>>) -> Result<bool, String> {
	let state_opt = state.lock().await;
	Ok(state_opt.is_some())
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
	let owner = act_wallet.request(None)?;

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

	act_wallet.receive(received_balance, token_id, genesis_spend, &owner)?;

	Ok(format!("{:x}", token_id))
}

#[tauri::command]
async fn request(token_id: String, state: State<'_, Mutex<Option<AppState>>>) -> Result<String, String> {
	let mut state_opt = state.lock().await;
	let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

	let act_wallet = &mut state.act_wallet;

	let bytes: [u8; 32] = hex::decode(token_id).map_err(|e| format!("{}", e))?
		.try_into().map_err(|_| "Wrong length".to_string())?;

	act_wallet.request(Some(XorName(bytes))).map(|key| key.to_hex())
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

fn describe_balances(results: &Vec<(XorName, Result<TokenInfo, String>, Result<U256, String>)>) -> Result<HashMap<String, (String, String)>, HashMap<String, (String, String)>> {
	results.iter()
		.fold(Ok(HashMap::<String, (String, String)>::new()), |balances_res: Result<HashMap<String, (String, String)>, HashMap<String, (String, String)>>, (token_id, info_res, balance_res)| {
			let entry_res = match info_res {
				Err(err) => {
					let len = match balances_res {
						Ok(ref b) => b.len(),
						Err(ref b) => b.len(),
					};
					Err(
						(format!("{:x}", token_id), (
							format!("({}) {}", len, err),
							"Token info error".to_string()
						))
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
						Ok(balance) => Ok((format!("{:x}", token_id), (info.symbol.clone(), balance))),
						Err(e) => Err((format!("{:x}", token_id), (info.symbol.clone(), e))),
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
		})
}

#[tauri::command]
async fn act_balances(state: State<'_, Mutex<Option<AppState>>>) -> Result<HashMap<String, (String, String)>, String> {
	let state_opt = state.lock().await;
	let state = state_opt.as_ref().ok_or("Not connected.")?;
	let wallet = &state.act_wallet;
	let client = &state.client;

	let balances = wallet.balance_total();
	println!("balances: {:?}", balances);

	let balances_results = stream::iter(balances.iter())
		.then(|(token_id, balance_res)| async move {
			let info_res = client.act_token_info(token_id).await;

			(*token_id, info_res, balance_res.clone())
		}).collect().await;

	describe_balances(&balances_results)
		.map_err(|balances_errors: HashMap<String, (String, String)>| format!("{:?}", balances_errors))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.invoke_handler(tauri::generate_handler![
			connect,
			is_connected,
			create_token,
			request,
			balance,
			act_balances,
		])
		.setup(|app| {
			app.manage(Mutex::new(None::<AppState>));
			if let Some(window) = app.webview_windows().iter().next() {
				window.1.set_theme(Some(Theme::Dark))?;
			}
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_describe_balances() {
		let mut balances = Vec::<(Result<TokenInfo, String>, Result<U256, String>)>::new();

		balances.push((Ok(TokenInfo {
			symbol: "EACT".to_string(),
			name: "Example Autonomi Community Token".to_string(),
			decimals: 18,

		}), U256::from_str_radix("10_000_000_000000_000000_000000", 10).map_err(|e| format!("{}", e)) ));

		assert_eq!(describe_balances(&balances), Ok(HashMap::from([
			("EACT".to_string(), "10000000.0".to_string()),
		])));

		balances.push((Ok(TokenInfo {
			symbol: "EACT2".to_string(),
			name: "Example Autonomi Community Token".to_string(),
			decimals: 18,

		}), Err("Some example error".to_string()) ));

		assert_eq!(describe_balances(&balances), Err(HashMap::from([
			("EACT".to_string(), "10000000.0".to_string()),
			("EACT2".to_string(), "Some example error".to_string()),
		])));

		balances.push((Err("Some example error".to_string()), Err("Some other example error".to_string()) ));

		assert_eq!(describe_balances(&balances), Err(HashMap::from([
			("EACT".to_string(), "10000000.0".to_string()),
			("EACT2".to_string(), "Some example error".to_string()),
			("(2) Some example error".to_string(), "Token info error".to_string()),
		])));
	}

	#[test]
	fn sk_generation_from_evm_privkey() {

		let sk = SecretKey::from_bytes(sn_bls_ckd::derive_master_sk(
			hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").expect("Decode EVM privkey")[0..32].try_into().unwrap()
		).expect("Wrong bytes").serialize().into()).expect("Wrong bytes");

		assert_eq!("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b", sk.to_hex());

		let sk = SecretKey::from_hex("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b").expect("Wrong hex");

		assert_eq!("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b", sk.to_hex());

		let sk = SecretKey::from_bytes(
			hex::decode("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b").expect("Decode EVM privkey")[0..32].try_into().unwrap()
		).expect("Wrong bytes");

		assert_eq!("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b", sk.to_hex());
	}

}
