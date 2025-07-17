use ant_act::{ActExt, TokenInfo, Wallet as ActWallet, WalletExt};
use autonomi::{
    client::payment::PaymentOption, Client, GraphEntry, GraphEntryAddress, PublicKey, SecretKey,
    Wallet, XorName,
};
use futures::{lock::Mutex, stream, FutureExt, StreamExt};
use ruint::aliases::U256;
use sn_curv::elliptic::curves::ECScalar;
use std::collections::HashMap;
use tauri::{Manager, State, Theme};

struct AppState {
    client: Client,
    wallet: Wallet,
    sk: SecretKey,
    act_wallet: ActWallet,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Network {
    Local,
    Alpha,
    Main,
}

#[tauri::command]
async fn connect(
    network: Network,
    evm_pk: Option<String>,
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<String, String> {
    let mut state = state.lock().await;

    if state.is_none() {
        let mut client = match network {
            Network::Local => Client::init_local().await,
            Network::Main => Client::init().await,
            Network::Alpha => Client::init_alpha().await,
        }
        .map_err(|e| format!("{}", e))?;

        let evm_pk = evm_pk.unwrap_or(SecretKey::random().to_hex()); // bls secret key can be used as eth privkey
        let evm_wallet = Wallet::new_from_private_key(client.evm_network().clone(), &evm_pk)
            .map_err(|e| format!("{}", e))?;

        println!("EVM Address: {}", evm_wallet.address());

        println!(
            "balance: {:?}",
            Option::zip(
                evm_wallet.balance_of_tokens().await.ok(),
                evm_wallet.balance_of_gas_tokens().await.ok()
            )
        );

        let evm_pk = if &evm_pk[0..2] == "0x" {
            &evm_pk[2..]
        } else {
            &evm_pk
        };

        let sk = SecretKey::from_bytes(
            sn_bls_ckd::derive_master_sk(
                hex::decode(evm_pk).map_err(|e| format!("{}", e))?[0..32]
                    .try_into()
                    .unwrap(),
            )
            .expect("Wrong bytes")
            .serialize()
            .into(),
        )
        .expect("Wrong bytes");
        println!("sk: {:.4}(...)", sk.to_hex());

        let client_clone = client.clone();
        let evm_wallet_clone = evm_wallet.clone();
        let sk_clone = sk.clone();

        let act_wallet = client_clone
            .act_wallet_get(&sk_clone)
            .then(|w_opt_res| async move {
                println!("W: {w_opt_res:?}");
                if let Ok(None) = w_opt_res {
                    let w = ActWallet::new(sk.public_key());
                    client
                        .act_wallet_save(&w, &sk, &PaymentOption::from(evm_wallet))
                        .await?;
                    Ok(Some(w))
                } else {
                    w_opt_res
                }
            })
            .await
            .map_err(|e| format!("{e}"))?
            .ok_or("Wallet could not be loaded nor created".to_string())?;

        *state = Some(AppState {
            client: client_clone,
            wallet: evm_wallet_clone,
            sk: sk_clone,
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
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<String, String> {
    let mut state_opt = state.lock().await;
    let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

    let client = &mut state.client;
    let evm_wallet = state.wallet.clone();
    let act_wallet = &mut state.act_wallet;
    let sk = &state.sk;

    let owner = act_wallet.request(None)?;
    let _ = client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet.clone()))
        .await?;

    let total_supply = U256::from_str_radix(&total_supply, 10).map_err(|e| format!("{}", e))?;
    let (genesis_spend, token_id) = client
        .act_create(
            name,
            symbol,
            decimals,
            total_supply,
            owner,
            &PaymentOption::from(evm_wallet.clone()),
        )
        .await?;

    let received_balance = client.act_balance(&owner, vec![genesis_spend]).await?;

    act_wallet.receive(received_balance, token_id, genesis_spend)?;
    let _ = client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet))
        .await?;

    Ok(format!("{:x}", token_id))
}

fn parse_xorname(xorname_str: &str) -> Result<XorName, String> {
    let bytes: [u8; 32] = hex::decode(xorname_str)
        .map_err(|e| format!("{}", e))?
        .try_into()
        .map_err(|_| "Wrong length".to_string())?;

    Ok(XorName(bytes))
}

#[tauri::command]
async fn request(
    token_id: String,
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<String, String> {
    let mut state_opt = state.lock().await;
    let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

    let client = &mut state.client;
    let evm_wallet = state.wallet.clone();
    let act_wallet = &mut state.act_wallet;
    let sk = &state.sk;

    let token_id = parse_xorname(&token_id)?;

    client.act_token_info(&token_id).await?;

    let public_key = act_wallet.request(Some(token_id))?;
    client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet))
        .await?;

    Ok(public_key.to_hex())
}

#[tauri::command]
async fn pay(
    token_id: String,
    amount: String,
    to: String,
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<String, String> {
    let mut state_opt = state.lock().await;
    let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

    let client = &mut state.client;
    let evm_wallet = &state.wallet;
    let act_wallet = &mut state.act_wallet;
    let sk = &state.sk;

    let token_id: XorName = parse_xorname(&token_id)?;

    let info = client.act_token_info(&token_id).await?;
    let amount: U256 = Decimal::from_string(amount, info.decimals)?;

    let to: PublicKey = PublicKey::from_hex(&to).map_err(|e| format!("{}", e))?;

    let payer_sk = sk.derive_child(
        &act_wallet
            .index_of_token(token_id)
            .ok_or("Key not found".to_string())?
            .to_be_bytes::<32>(),
    );

    let (input_spends, sum, rest_key) = act_wallet.take_to_spend(token_id.clone())?;
    let _ = client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet.clone()))
        .await?;
    println!("Inputs: {:?}", (&input_spends, sum));

    let rest_amount = sum
        .checked_sub(amount) // arg?, arg
        .ok_or("Overflow".to_string())?;

    let spend = GraphEntry::new(
        &payer_sk,    // arg
        input_spends, // arg
        token_id.0,   // arg
        vec![
            (to, amount.to_be_bytes()),            // arg
            (rest_key, rest_amount.to_be_bytes()), // arg
        ],
    );
    // TODO: validate

    let (_paid, spend_address) = client
        .graph_entry_put(spend, PaymentOption::from(evm_wallet.clone()))
        .await // arg
        .map_err(|e| format!("{}", e))?;

    println!("Spend GraphEntry: {}", spend_address);

    act_wallet.receive(rest_amount, token_id, *spend_address.owner())?;
    let _ = client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet.clone()))
        .await?;
    println!("Payer Wallet: {:?}", act_wallet);

    Ok(spend_address.owner().to_hex())
}

#[tauri::command]
async fn receive(
    spend_address: String,
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<(), String> {
    let mut state_opt = state.lock().await;
    let state: &mut AppState = state_opt.as_mut().ok_or("Not connected.")?;

    let client = &mut state.client;
    let evm_wallet = state.wallet.clone();
    let act_wallet = &mut state.act_wallet;
    let sk = &state.sk;

    println!("Receive spend: {}", spend_address);
    let spend_address =
        GraphEntryAddress::from_hex(&spend_address).map_err(|e| format!("{}", e))?;

    let spend = client
        .graph_entry_get(&spend_address)
        .await
        .map_err(|e| format!("{}", e))?;
    println!("Receive spend GE: {:?}", spend);

    let token_id = XorName(spend.content);
    let pk = act_wallet
        .pk_of_token(token_id)
        .ok_or("Payment has not been requested".to_string())?;
    println!("Receive pk: {:.4}(...)", pk.to_hex());

    let (amount, overflow, empty) = spend
        .descendants
        .iter()
        .filter(|(public_key, _)| *public_key == pk)
        .map(|(_, data)| U256::from_be_bytes::<32>(*data))
        .fold(
            (U256::ZERO, false, true),
            |(sum, any_overflow, _empty), n| {
                let (added, this_overflow) = sum.overflowing_add(n);
                (added, any_overflow || this_overflow, false)
            },
        );
    println!(
        "Receive (amount, overflow, empty): {:?}",
        (&amount, &overflow, &empty)
    );

    if empty {
        return Err("Could not find your Public Key in the spend".to_string());
    }

    if overflow {
        return Err("Overflow".to_string());
    }

    if act_wallet.received_spend(token_id, *spend_address.owner()) {
        return Err("Already received this spend".to_string());
    }

    act_wallet.receive(amount, token_id, *spend_address.owner())?;
    println!("Receive wallet: {:?}", act_wallet);
    let _ = client
        .act_wallet_save(&act_wallet, sk, &PaymentOption::from(evm_wallet))
        .await?;

    Ok(())
}

#[tauri::command]
async fn balance(state: State<'_, Mutex<Option<AppState>>>) -> Result<(String, String), String> {
    let state_opt = state.lock().await;
    let state = state_opt.as_ref().ok_or("Not connected.")?;

    let wallet = state.wallet.clone();

    let ant = format!(
        "{}",
        wallet
            .balance_of_tokens()
            .await
            .map_err(|e| format!("{}", e))?
    );
    let eth = format!(
        "{}",
        wallet
            .balance_of_gas_tokens()
            .await
            .map_err(|e| format!("{}", e))?
    );

    Ok((ant, eth))
}

fn describe_balances(
    results: &Vec<(XorName, Result<TokenInfo, String>, Result<U256, String>)>,
) -> Result<HashMap<String, (String, String)>, HashMap<String, (String, String)>> {
    results.iter().fold(
        Ok(HashMap::<String, (String, String)>::new()),
        |balances_res: Result<
            HashMap<String, (String, String)>,
            HashMap<String, (String, String)>,
        >,
         (token_id, info_res, balance_res)| {
            let entry_res = match info_res {
                Err(err) => {
                    let len = match balances_res {
                        Ok(ref b) => b.len(),
                        Err(ref b) => b.len(),
                    };
                    Err((
                        format!("{:x}", token_id),
                        (format!("({}) {}", len, err), "Token info error".to_string()),
                    ))
                }
                Ok(info) => {
                    let balance_res = balance_res
                        .clone()
                        .and_then(|balance| Decimal::to_string(balance, info.decimals));

                    match balance_res {
                        Ok(balance) => {
                            Ok((format!("{:x}", token_id), (info.symbol.clone(), balance)))
                        }
                        Err(e) => Err((format!("{:x}", token_id), (info.symbol.clone(), e))),
                    }
                }
            };

            match balances_res {
                Err(mut balances_errors) => {
                    let entry = entry_res.unwrap_or_else(|e| e);
                    balances_errors.insert(entry.0, entry.1);
                    Err(balances_errors)
                }
                Ok(mut balances) => match entry_res {
                    Ok(entry) => {
                        balances.insert(entry.0, entry.1);
                        Ok(balances)
                    }
                    Err(entry) => {
                        balances.insert(entry.0, entry.1);
                        Err(balances)
                    }
                },
            }
        },
    )
}

#[tauri::command]
async fn act_balances(
    state: State<'_, Mutex<Option<AppState>>>,
) -> Result<HashMap<String, (String, String)>, String> {
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
        })
        .collect()
        .await;

    describe_balances(&balances_results).map_err(
        |balances_errors: HashMap<String, (String, String)>| format!("{:?}", balances_errors),
    )
}

struct Decimal;

impl Decimal {
    fn to_string(value: U256, decimals: u8) -> Result<String, String> {
        U256::from(10)
            .checked_pow(U256::from(decimals))
            .map(|divisor| {
                let (intg, rem) = value.div_rem(divisor);
                let dec = &format!("{}", rem.saturating_add(divisor))[1..]; // remove "1" from beginning
                let dec = dec.trim_end_matches('0'); // remove trailing zeros
                let dec = match dec.len() {
                    0 => "0",
                    _ => dec,
                };
                format!("{}.{}", intg, dec)
            })
            .ok_or("Overflow".to_string())
    }

    fn from_string(input: String, decimals: u8) -> Result<U256, String> {
        let mut input = input.split('.');
        let decimals: usize = decimals.into();

        let intg = input.next().ok_or("Wrong input".to_string())?;
        let intg = U256::from_str_radix(intg, 10).map_err(|e| format!("{}", e))?;

        let dec = format!("{:0<decimals$}", input.next().unwrap_or(""));
        let dec: String = dec.chars().take(decimals).collect();
        let dec = U256::from_str_radix(&dec, 10).map_err(|e| format!("{}", e))?;

        U256::from(10)
            .checked_pow(U256::from(decimals))
            .and_then(|multiplier| intg.checked_mul(multiplier))
            .and_then(|no_rest| no_rest.checked_add(dec))
            .ok_or("Overflow".to_string())
    }
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
            pay,
            receive,
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
        let mut balances = Vec::<(XorName, Result<TokenInfo, String>, Result<U256, String>)>::new();

        let some_xorname1 = "6150aa3c2c43e458a03b773b520ba8aa1f3a3eef6db88ba44b31734932cc1749";
        let some_xorname2 = "6150aa3c2c43e458a03b773b520ba8aa1f3a3eef6db88ba44b31734932000000";
        let some_xorname3 = "6150aa3c2c43e458a03b773b520ba8aa1f3a3eef6db88b000000000000000000";

        balances.push((
            parse_xorname(some_xorname1).expect("Xorname should parse"),
            Ok(TokenInfo {
                symbol: "EACT".to_string(),
                name: "Example Autonomi Community Token".to_string(),
                decimals: 18,
            }),
            U256::from_str_radix("10_000_000_000000_000000_000000", 10)
                .map_err(|e| format!("{}", e)),
        ));

        assert_eq!(
            describe_balances(&balances),
            Ok(HashMap::from([(
                some_xorname1.to_string(),
                ("EACT".to_string(), "10000000.0".to_string())
            ),]))
        );

        balances.push((
            parse_xorname(some_xorname2).expect("Xorname should parse"),
            Ok(TokenInfo {
                symbol: "EACT2".to_string(),
                name: "Example Autonomi Community Token".to_string(),
                decimals: 18,
            }),
            Err("Some example error".to_string()),
        ));

        assert_eq!(
            describe_balances(&balances),
            Err(HashMap::from([
                (
                    some_xorname1.to_string(),
                    ("EACT".to_string(), "10000000.0".to_string())
                ),
                (
                    some_xorname2.to_string(),
                    ("EACT2".to_string(), "Some example error".to_string())
                ),
            ]))
        );

        balances.push((
            parse_xorname(some_xorname3).expect("Xorname should parse"),
            Err("Some example error".to_string()),
            Err("Some other example error".to_string()),
        ));

        assert_eq!(
            describe_balances(&balances),
            Err(HashMap::from([
                (
                    some_xorname1.to_string(),
                    ("EACT".to_string(), "10000000.0".to_string())
                ),
                (
                    some_xorname2.to_string(),
                    ("EACT2".to_string(), "Some example error".to_string())
                ),
                (
                    some_xorname3.to_string(),
                    (
                        "(2) Some example error".to_string(),
                        "Token info error".to_string()
                    )
                ),
            ]))
        );
    }

    #[test]
    fn sk_generation_from_evm_privkey() {
        let sk = SecretKey::from_bytes(
            sn_bls_ckd::derive_master_sk(
                hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
                    .expect("Decode EVM privkey")[0..32]
                    .try_into()
                    .unwrap(),
            )
            .expect("Wrong bytes")
            .serialize()
            .into(),
        )
        .expect("Wrong bytes");

        assert_eq!(
            "4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b",
            sk.to_hex()
        );

        let sk =
            SecretKey::from_hex("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b")
                .expect("Wrong hex");

        assert_eq!(
            "4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b",
            sk.to_hex()
        );

        let sk = SecretKey::from_bytes(
            hex::decode("4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b")
                .expect("Decode EVM privkey")[0..32]
                .try_into()
                .unwrap(),
        )
        .expect("Wrong bytes");

        assert_eq!(
            "4f7eedb7b093537a4402daa0769dfca018520ee3ea2107338d89cbfcc312451b",
            sk.to_hex()
        );
    }

    #[test]
    fn decimal_str() {
        assert_eq!(
            Decimal::to_string(U256::from(10000), 3),
            Ok("10.0".to_string())
        );
        assert_eq!(
            Decimal::from_string("10.0".to_string(), 3),
            Ok(U256::from(10000))
        );

        assert_eq!(Decimal::to_string(U256::ZERO, 3), Ok("0.0".to_string()));
        assert_eq!(Decimal::from_string("0.0".to_string(), 3), Ok(U256::ZERO));

        assert_eq!(
            Decimal::from_string("0.123456".to_string(), 3),
            Ok(U256::from(123))
        );
    }
}
