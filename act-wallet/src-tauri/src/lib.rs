use futures::lock::Mutex;
use tauri::{AppHandle, State, Manager};
use autonomi::{Client};


struct AppState {
	client: Option<Client>,
}


#[tauri::command]
async fn connect(peer: Option<String>, state: State<'_, Mutex<AppState>>) -> Result<(), String> {
	let mut state = state.lock().await;

	if state.client.is_none() {
		let client = Client::init_local().await.map_err(|e| format!("{}", e))?;
		state.client = Some(client);
	}

	Ok(())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.invoke_handler(tauri::generate_handler![connect])
		.setup(|app| {
			app.manage(Mutex::new(AppState {
				client: None,
			}));
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
