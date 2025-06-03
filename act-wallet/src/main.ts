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
    const currentPk = await invoke("connect", {
      network: network,
      evmPk: pk,
    });
    console.log("Connected.");

    if (pk === null) {
//      await navigator.clipboard.writeText(currentPk);
      window.alert("Your EVM Private Key, keep it safely: " + currentPk);
    }

    await refresh();
  }
}

async function createToken() {
  const nameInputEl = document.querySelector("#create-token-name input");
  const name = nameInputEl.value;
  
  const symbolInputEl = document.querySelector("#create-token-symbol input");
  const symbol = symbolInputEl.value;
  
  const supplyInputEl = document.querySelector("#create-token-supply input");
  const supply = supplyInputEl.value;

  const decimalsInputEl = document.querySelector("#create-token-decimals input");
  const decimals = decimalsInputEl.value;

  try {
    const tokenId = await invoke("create_token", {
      name: name,
      symbol: symbol,
      decimals: parseInt(decimals),
      totalSupply: supply,
    });

    message("Token ID: " + tokenId, "create-token");

  } catch (e) {
    error("" + e, "create-token");
  }
  await balance();
}

async function request() {
  const tokenId = document.querySelector("#request-token-id input")?.value;
  console.log("tokenId: ", tokenId);

  try {
    const publicKey = await invoke("request", {
      tokenId: tokenId,
    });

    message("Public Key: " + publicKey, "request");

  } catch (e) {
    error("" + e, "request");
  }
  await balance();
}

async function pay() {
  const tokenId = document.querySelector("#pay-token-id input")?.value;
  const amount = document.querySelector("#pay-amount input")?.value;
  const to = document.querySelector("#pay-to input")?.value;

  try {
    const spendAddress = await invoke("pay", {
      tokenId: tokenId,
      amount: amount,
      to: to,
    });

    message("Crated spend: " + spendAddress, "pay");

  } catch (e) {
    error("" + e, "pay");
  }
  await balance();
}

async function receive() {
  const spendAddress = document.querySelector("#receive-spend input")?.value;

  try {
    await invoke("receive", {
      spendAddress: spendAddress,
    });

    message("Tokens received.", "receive");

  } catch (e) {
    error("" + e, "receive");
  }
  await balance();
}

function balanceHtml(actBalance: object): string {
  let balHtml = "";
  for (let tokenId in actBalance) {
    balHtml += `<strong>${actBalance[tokenId][0]}</strong>: ${actBalance[tokenId][1]}, `;
  }
  return balHtml.substring(0, balHtml.length - 2); // remove last comma
}

async function balance() {
  document.getElementById("balance").hidden = false;
  let balanceEl = document.querySelector("#balance p");

  let bal = await invoke("balance");
  console.log(bal);
  if (Array.isArray(bal)) {
    bal = balanceHtml({ "fakeTokenId1": ["ATTOS", bal[0]], "fakeTokenId2": ["WEI", bal[1]] });
  }
  const actBalance = await invoke("act_balances");
  console.log("actBalance");
  console.log(actBalance);

  populateTokenIdSelect(document.querySelector("#request-token-id select"), actBalance);
  populateTokenIdSelect(document.querySelector("#pay-token-id select"), actBalance);

  let actBalanceHtml = "–";
  if (typeof actBalance === 'object' && Object.keys(actBalance).length > 0) {
    actBalanceHtml = balanceHtml(actBalance);
  }
  
  balanceEl.innerHTML = "<dt>EVM balance (gas)</dt> <dd>" + bal + "</dd> <br />"
    + "<dt>ACT balance</dt> <dd>" + actBalanceHtml + "</dd>";
}

function optionHtml(value: string, text: string): Element {
    let option = document.createElement("option");
    option.value = value;
    option.innerHTML = text;
    return option;
}

function populateTokenIdSelect(select: Element, actBalance: object) {
  select.replaceChildren(); // clear
  select.append(optionHtml("", "(clear)"));
  for (let tokenId in actBalance) {
    select.append(optionHtml(tokenId, actBalance[tokenId][0] + " (" + tokenId.substring(0, 6) + "...)"));
  }
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
    console.log(`(${afterId}): ${text}`);
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
    console.error(`(${afterId}): ${text}`);
  }
  errEl.innerHTML = text;
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("balance").hidden = true;
  refresh();

  // connect

  document.querySelector("#main-connect-button")?.addEventListener("click", (e) => connect("Main"));
  document.querySelector("#local-connect-button")?.addEventListener("click", (e) => connect("Local"));
  document.querySelector("#alpha-connect-button")?.addEventListener("click", (e) => connect("Alpha"));

  // menu

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

  // request

  document.querySelector("#request-token-id select")?.addEventListener("change", (e) => {
    document.querySelector("#request-token-id input").value = e.target.value;
  });

  document.querySelector("#request button")?.addEventListener("click", async (e) => {
    await request();
  });

  // pay

  document.querySelector("#pay-token-id select")?.addEventListener("change", (e) => {
    document.querySelector("#pay-token-id input").value = e.target.value;
  });

  document.querySelector("#pay button")?.addEventListener("click", async (e) => {
    await pay();
  });

  // receive

  document.querySelector("#receive button")?.addEventListener("click", async (e) => {
    await receive();
  });

  // create token

  document.querySelector("#create-token button")?.addEventListener("click", async (e) => {
    await createToken();
  });
});
