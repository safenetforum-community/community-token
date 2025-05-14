import { invoke } from "@tauri-apps/api/core";

async function connect(local: boolean) {
  const pkInputEl = document.querySelector("#pk-input");

  if (pkInputEl) {
    const pk = pkInputEl.value || null;
    console.log("Connecting Autonomi...");
    await invoke("connect", {
      local: local,
      evmPk: pk,
    });
    console.log("Connected.");

    await balance();
    document.getElementById("connect").hidden = true;
  }
}

async function createToken() {
  const nameInputEl = document.querySelector("#create-token name input");
  const name = nameInputEl.value;
  
  const symbolInputEl = document.querySelector("#create-token symbol input");
  const symbol = symbolInputEl.value;
  
  const supplyInputEl = document.querySelector("#create-token supply input");
  const supply = supplyInputEl.value;

  const decimalsInputEl = document.querySelector("#create-token decimals input");
  const decimals = decimalsInputEl.value;

  try {
    const tokenId = await invoke("create_token", {
      name: name,
      symbol: symbol,
      decimals: decimals,
      totalSupply: supply,
    });

    message("Token ID: " + tokenId, "create-token")

  } catch (e) {
    error("" + e, "create-token");
  }
}

async function balance() {
  let balanceEl = document.querySelector("#balance");
  if !balanceEl {
    balanceEl = document.createElement("p");
    balanceEl.id = "balance";
    document.getElementById("connect").after(balanceEl);
  }

  const balance = await invoke("balance");
  balanceEl.innerHTML = "Balance: " + balance;
}

function message(text: string, afterId: string) {
  let msgEl = document.querySelector("#{afterId} ~ .message");
  if !msgEl {
    msgEl = document.createElement("p");
    msgEl.className = "message";
    document.getElementById(afterId).after(msgEl);
  }
  msgEl.innerHTML = text;
}

function error(text: string, afterId: string) {
  let errEl = document.querySelector("#{afterId} ~ .error");
  if !errEl {
    errEl = document.createElement("p");
    errEl.className = "error";
    document.getElementById(afterId).after(errEl);
  }
  errEl.innerHTML = text;
}

window.addEventListener("DOMContentLoaded", () => {

  document.querySelector("#main-connect-button")?.addEventListener("click", (e) => connect(false));
  document.querySelector("#local-connect-button")?.addEventListener("click", (e) => connect(true));

  document.querySelector("#create-token button")?.addEventListener("click", (e) => createToken());
});
