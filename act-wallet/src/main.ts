import { invoke } from "@tauri-apps/api/core";

async function refresh() {
  if (await invoke("is_connected")) {
    await balance();
    document.getElementById("connect").hidden = true;
  }
}

async function connect(network: string) {
  const pkInputEl = document.querySelector("#pk-input");

  if (pkInputEl) {
    const pk = pkInputEl.value || null;
    console.log("Connecting Autonomi...");
    await invoke("connect", {
      network: network,
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

function toHtml(obj: object) {
  let balHtml = "";
  for (let symbol in obj) {
    balHtml += `<strong>${symbol}</strong>: ${obj[symbol]}, `;
  }
  return balHtml.substring(0, balHtml.length - 2); // remove last comma
}

async function balance() {
  document.getElementById("balance").hidden = false;
  let balanceEl = document.querySelector("#balance p");

  let bal = await invoke("balance");
  console.log(bal);
  if (Array.isArray(bal)) {
    bal = toHtml({ "ATTOS": bal[0], "WEI": bal[0] });
  }
  const actBalance = await invoke("act_balances");
  console.log("actBalance");
  console.log(actBalance);

  let actBalanceHtml = "–";
  if (typeof actBalance === 'object' && Object.keys(actBalance).length > 0) {
    actBalanceHtml = toHtml(actBalance);
  }
  
  balanceEl.innerHTML = "<dt>EVM balance</dt> <dd>" + bal + "</dd> <br />"
    + "<dt>ACT balance</dt> <dd>" + actBalanceHtml + "</dd>";
}


function message(text: string, afterId: string) {
  let msgEl = document.querySelector(`#${afterId} .message`);
  if (!msgEl) {
    msgEl = document.createElement("p");
    msgEl.className = "message";
    document.getElementById(afterId).append(msgEl);
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
  let errEl = document.querySelector(`#${afterId} .error`);
  if (!errEl) {
    errEl = document.createElement("p");
    errEl.className = "error";
    document.getElementById(afterId).append(errEl);
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

  document.querySelector("#main-connect-button")?.addEventListener("click", (e) => connect("Main"));
  document.querySelector("#local-connect-button")?.addEventListener("click", (e) => connect("Local"));
  document.querySelector("#alpha-connect-button")?.addEventListener("click", (e) => connect("Alpha"));

  let menuButtons = document.querySelectorAll("#menu li");
  for (const button of menuButtons) {
    button.addEventListener("click", (e) => {
      let target = button.getAttribute("data-targetid");
  
      document.querySelectorAll("#panel .subpanel").forEach((subpanel) => subpanel.hidden = true);
      document.getElementById(target).hidden = false;
  
      document.querySelector("#panel #menu .active").setAttribute("class", "");
      button.setAttribute("class", "active");
    });
  }
  

  document.querySelector("#create-token button")?.addEventListener("click", async (e) => {
    await createToken();
  });
});
