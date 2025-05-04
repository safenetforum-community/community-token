import { invoke } from "@tauri-apps/api/core";

let peerInputEl: HTMLInputElement | null;

async function connect() {
  if (peerInputEl) {
    const peer = peerInputEl.value || null;
    console.log("Connecting Autonomi...");
    await invoke("connect", {
      peer: peer,
    });
    console.log("Connected.");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  peerInputEl = document.querySelector("#peer-input");

  document.querySelector("#connect-button")?.addEventListener("click", (e) => connect());
});
