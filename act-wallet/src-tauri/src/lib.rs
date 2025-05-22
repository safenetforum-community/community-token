use futures::lock::Mutex;
use ruint::aliases::U256;
use sn_curv::elliptic::curves::ECScalar;
use tauri::{AppHandle, State, Manager};
use autonomi::{Client, SecretKey, Wallet,
	client::payment::PaymentOption};
use ant_act::{ActExt};


struct AppState {
	client: Client,
	wallet: Wallet,
	sk: SecretKey,
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

		*state = Some(AppState {
			client,
			wallet,
			sk
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
	let state_opt = state.lock().await;
	let state = state_opt.as_ref().ok_or("Not connected.")?;

	let client = state.client.clone();
	let sk = state.sk.clone();
	let evm_wallet = state.wallet.clone();

	let with_wallet = PaymentOption::from(evm_wallet);

	let mut act_wallet = ant_act::Wallet::new(sk);
//	let act_wallet = client.act_wallet(sk);
	let owner = act_wallet.request(vec![1].into());

	let total_supply = U256::from_str_radix(&total_supply, 10).map_err(|e| format!("{}", e))?;
	let genesis_spend = client.act_create(
		name,
		symbol,
		decimals,
		total_supply,
		owner,
		&with_wallet,
	).await.map_err(|e| format!("{}", e))?;

	Ok(genesis_spend.0.to_hex())
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


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.invoke_handler(tauri::generate_handler![
			connect,
			create_token,
			balance,
		])
		.setup(|app| {
			app.manage(Mutex::new(None::<AppState>));
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
