import { invoke } from "@tauri-apps/api/core";

let pkInputEl: HTMLInputElement | null;

async function connect(local: boolean) {
  if (pkInputEl) {
    const pk = pkInputEl.value || null;
    console.log("Connecting Autonomi...");
    await invoke("connect", {
      local: local,
      evmPk: pk,
    });
    console.log("Connected.");

    let balance = await invoke("balance");
    let balanceEl = document.createElement("p");
    balanceEl.innerHTML = "Balance: " + balance;

    document.getElementById("connect").after(balanceEl);
    document.getElementById("connect").hidden = true;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  pkInputEl = document.querySelector("#pk-input");

  document.querySelector("#main-connect-button")?.addEventListener("click", (e) => connect(false));
  document.querySelector("#local-connect-button")?.addEventListener("click", (e) => connect(true));
});
