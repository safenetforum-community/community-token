import { invoke } from "@tauri-apps/api/core";

function for_existing_element(id: string, f: (el: Element) => void) {
  let element = document.getElementById(id);
  if (element) {
    f(element);
  }
}

function for_existing_query(query: string, f: (el: Element) => void) {
  let element = document.querySelector(query);
  if (element) {
    f(element);
  }
}

async function refresh() {
  if (await invoke("is_connected")) {
    await balance();
    for_existing_element("connect", (el) => (el as HTMLElement).hidden = true);
  }
}

async function connect(network: string) {
  const pkInputEl: HTMLInputElement | null = document.querySelector("#pk-input");

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
  const name = (document.querySelector("#create-token-name input") as HTMLInputElement)?.value;
  const symbol = (document.querySelector("#create-token-symbol input") as HTMLInputElement)?.value;
  const supply = (document.querySelector("#create-token-supply input") as HTMLInputElement)?.value;
  const decimals = (document.querySelector("#create-token-decimals input") as HTMLInputElement)?.value;

  try {
    const tokenId = await invoke("create_token", {
      name: name,
      symbol: symbol,
      decimals: parseInt(decimals),
      totalSupply: supply,
    });

    message("Token ID: " + tokenId, "create-token");

  } catch (err) {
    error("" + err, "create-token");
  }
  await balance();
}

async function request() {
  const tokenId = (document.querySelector("#request-token-id input") as HTMLInputElement)?.value;
  console.log("tokenId: ", tokenId);

  try {
    const publicKey = await invoke("request", {
      tokenId: tokenId,
    });

    message("Public Key: " + publicKey, "request");

  } catch (err) {
    error("" + err, "request");
  }
  await balance();
}

async function pay() {
  const tokenId = (document.querySelector("#pay-token-id input") as HTMLInputElement)?.value;
  const amount = (document.querySelector("#pay-amount input") as HTMLInputElement)?.value;
  const to = (document.querySelector("#pay-to input") as HTMLInputElement)?.value;

  try {
    const spendAddress = await invoke("pay", {
      tokenId: tokenId,
      amount: amount,
      to: to,
    });

    message("Crated spend: " + spendAddress, "pay");

  } catch (err) {
    error("" + err, "pay");
  }
  await balance();
}

async function receive() {
  const spendAddress = (document.querySelector("#receive-spend input") as HTMLInputElement)?.value;

  try {
    await invoke("receive", {
      spendAddress: spendAddress,
    });

    message("Tokens received.", "receive");

  } catch (err) {
    error("" + err, "receive");
  }
  await balance();
}

type ActBalance = {
  [tokenId: string]: [string, string]
};

function balanceHtml(actBalance: ActBalance): string {
  let balHtml = "";
  for (let tokenId in actBalance) {
    balHtml += `<strong>${actBalance[tokenId][0]}</strong>: ${actBalance[tokenId][1]}, `;
  }
  return balHtml.substring(0, balHtml.length - 2); // remove last comma
}

async function balance() {
  for_existing_element("balance", (el) => (el as HTMLElement).hidden = false);

  let bal = await invoke("balance");
  console.log(bal);
  if (Array.isArray(bal)) {
    bal = balanceHtml({ "fakeTokenId1": ["ATTOS", bal[0]], "fakeTokenId2": ["WEI", bal[1]] });
  }
  const actBalance: ActBalance = await invoke("act_balances");
  console.log("actBalance");
  console.log(actBalance);

  populateTokenIdSelect((document.querySelector("#request-token-id select") as Element), actBalance);
  populateTokenIdSelect((document.querySelector("#pay-token-id select") as Element), actBalance);

  let actBalanceHtml = "–";
  if (typeof actBalance === 'object' && Object.keys(actBalance).length > 0) {
    actBalanceHtml = balanceHtml(actBalance);
  }
  
  for_existing_query("#balance p", (balanceEl) => balanceEl.innerHTML =
    "<dt>EVM balance (gas)</dt> <dd>" + bal + "</dd> <br />"
    + "<dt>ACT balance</dt> <dd>" + actBalanceHtml + "</dd>");
}

function optionHtml(value: string, text: string): Element {
    let option = document.createElement("option");
    option.value = value;
    option.innerHTML = text;
    return option;
}

function populateTokenIdSelect(select: Element, actBalance: ActBalance) {
  select.replaceChildren(); // clear
  select.append(optionHtml("", "(clear)"));
  for (let tokenId in actBalance) {
    select.append(optionHtml(tokenId, actBalance[tokenId][0] + " (" + tokenId.substring(0, 6) + "...)"));
  }
}


function message(text: string, afterId: string) {
  let msgEl: HTMLElement | null = document.querySelector(`#${afterId} .message`);
  if (!msgEl) {
    msgEl = document.createElement("p");
    msgEl.className = "message";
    for_existing_element(afterId, (el) => el.append((msgEl as HTMLElement)));
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
  let errEl: HTMLElement | null = document.querySelector(`#${afterId} .error`);
  if (!errEl) {
    errEl = document.createElement("p");
    errEl.className = "error";
    for_existing_element(afterId, (el) => el.append((errEl as HTMLElement)));
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
  for_existing_element("balance", (el) => (el as HTMLElement).hidden = true);
  refresh();

  // connect

  document.querySelector("#main-connect-button")?.addEventListener("click", (_ev) => connect("Main"));
  document.querySelector("#local-connect-button")?.addEventListener("click", (_ev) => connect("Local"));
  document.querySelector("#alpha-connect-button")?.addEventListener("click", (_ev) => connect("Alpha"));

  // menu

  let menuButtons = document.querySelectorAll("#menu li");
  for (const button of menuButtons) {
    button.addEventListener("click", (_ev) => {
      let target = button.getAttribute("data-targetid");
  
      document.querySelectorAll("#panel .subpanel").forEach(
        (subpanel) => (subpanel as HTMLElement).hidden = true
      );
      if (target) {
        for_existing_element(target, (el) => (el as HTMLElement).hidden = false);
      }
  
      for_existing_query("#panel #menu .active", (el) => el.setAttribute("class", ""));
      button.setAttribute("class", "active");
    });
  }

  // request

  document.querySelector("#request-token-id select")?.addEventListener("change", (ev) => {
    for_existing_query("#request-token-id input",
      (el) => (el as HTMLInputElement).value = (ev.target as HTMLInputElement)?.value
    );
  });

  document.querySelector("#request button")?.addEventListener("click", async (_ev) => {
    await request();
  });

  // pay

  document.querySelector("#pay-token-id select")?.addEventListener("change", (ev) => {
    for_existing_query("#pay-token-id input",
      (el) => (el as HTMLInputElement).value = (ev.target as HTMLInputElement)?.value
    );
  });

  document.querySelector("#pay button")?.addEventListener("click", async (_ev) => {
    await pay();
  });

  // receive

  document.querySelector("#receive button")?.addEventListener("click", async (_ev) => {
    await receive();
  });

  // create token

  document.querySelector("#create-token button")?.addEventListener("click", async (_ev) => {
    await createToken();
  });
});
