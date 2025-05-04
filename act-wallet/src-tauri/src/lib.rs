use futures::lock::Mutex;
use tauri::{AppHandle, State, Manager};
use autonomi::{Client, SecretKey, Wallet};


struct AppState {
	client: Option<Client>,
	wallet: Option<Wallet>,
}


#[tauri::command]
async fn connect(local: bool, evm_pk: Option<String>, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
	let mut state = state.lock().await;

	if state.client.is_none() {
		let client = if local {
			Client::init_local().await
		} else {
			Client::init().await
		}.map_err(|e| format!("{}", e))?;


		let evm_pk = evm_pk.unwrap_or(SecretKey::random().to_hex()); // bls secret key can be used as eth privkey
		let wallet = Wallet::new_from_private_key(client.evm_network().clone(), &evm_pk)
			.map_err(|e| format!("{}", e))?;

		println!("EVM Address: {}", wallet.address());

		state.client = Some(client);
		state.wallet = Some(wallet);
	}

	Ok(())
}


#[tauri::command]
async fn balance(state: State<'_, Mutex<AppState>>) -> Result<(String, String), String> {
	let mut state = state.lock().await;

	let wallet = state.wallet.clone().ok_or("Not connected.")?;

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
			balance,
		])
		.setup(|app| {
			app.manage(Mutex::new(AppState {
				client: None,
				wallet: None,
			}));
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
