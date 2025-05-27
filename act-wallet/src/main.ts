import { invoke } from "@tauri-apps/api/core";

async function refresh() {
  if (await invoke("is_connected")) {
    await balance();
    document.getElementById("connect").hidden = true;
  }
}

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

    await refresh();
  }
}

async function createToken() {
  const nameInputEl = document.querySelector("#create-token #name input");
  const name = nameInputEl.value;
  
  const symbolInputEl = document.querySelector("#create-token #symbol input");
  const symbol = symbolInputEl.value;
  
  const supplyInputEl = document.querySelector("#create-token #supply input");
  const supply = supplyInputEl.value;

  const decimalsInputEl = document.querySelector("#create-token #decimals input");
  const decimals = decimalsInputEl.value;

  try {
    const tokenId = await invoke("create_token", {
      name: name,
      symbol: symbol,
      decimals: parseInt(decimals),
      totalSupply: supply,
    });

    message("Token ID: " + tokenId, "create-token")

    await balance();

  } catch (e) {
    error("" + e, "create-token");
  }
}

async function balance() {
  document.getElementById("balance").hidden = false;
  let balanceEl = document.querySelector("#balance p");

  const balance = await invoke("balance");
  const actBalance = await invoke("act_balances");
  console.log("actBalance");
  console.log(actBalance);

  let actBalanceHtml = "–";
  if (typeof actBalance === 'object') {
    actBalanceHtml = "";
    for (let symbol in actBalance) {
      actBalanceHtml += `<strong>${symbol}</strong>: ${actBalance[symbol]}, `;
    }
    actBalanceHtml = actBalanceHtml.substring(0, actBalanceHtml.length - 2); // remove last comma
  }
  
  balanceEl.innerHTML = "<dt>EVM balance</dt> <dd>" + balance + "</dd> <br />"
    + "<dt>ACT balance</dt> <dd>" + actBalanceHtml + "</dd>";
}

function switchTo(id: string) {
}

function message(text: string, afterId: string) {
  let msgEl = document.querySelector(`#${afterId} ~ .message`);
  if (!msgEl) {
    msgEl = document.createElement("p");
    msgEl.className = "message";
    document.getElementById(afterId).after(msgEl);
  }
  if (!text) {
    msgEl.hidden = true;
  } else {
    error("", afterId);
    msgEl.hidden = false;
  }
  msgEl.innerHTML = text;
}

function error(text: string, afterId: string) {
  let errEl = document.querySelector(`#${afterId} ~ .error`);
  if (!errEl) {
    errEl = document.createElement("p");
    errEl.className = "error";
    document.getElementById(afterId).after(errEl);
  }
  if (!text) {
    errEl.hidden = true;
  } else {
    message("", afterId);
    errEl.hidden = false;
  }
  errEl.innerHTML = text;
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("balance").hidden = true;
  refresh();

  document.querySelector("#main-connect-button")?.addEventListener("click", (e) => connect(false));
  document.querySelector("#local-connect-button")?.addEventListener("click", (e) => connect(true));

  let menuButtons = document.querySelectorAll("#menu li");
  for (const button of menuButtons) {
    button.addEventListener("click", (e) => {
      let target = e.target.getAttribute("data-targetid");
  
      document.querySelectorAll("#panel .subpanel").forEach((subpanel) => subpanel.hidden = true);
      document.getElementById(target).hidden = false;
  
      document.querySelector("#panel #menu .active").setAttribute("class", "");
      e.target.setAttribute("class", "active");
    });
  }
  

  document.querySelector("#create-token button")?.addEventListener("click", async (e) => {
    await createToken();
  });
});
