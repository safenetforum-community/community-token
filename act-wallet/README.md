# Autonomi Community Token Wallet

* [How to use ACT Wallet app (user instructions)](docs/USAGE.md)

## Testing

It's best to run the app from the terminal to see backend (Rust stdout) debug info.

You can run *Development Tools* in app by `CTRL + SHIFT + I` to see frontend logs (JavaScript console) etc.

## Running from sources

You will need *Rust*, *Node.js*, and *yarn*

```
yarn install
yarn tauri dev
```

If you want to run **second instance of the app**, you can change *"dev"* in *"scripts"* section of `package.json` after you run first instance from `vite` to `vite --port 1422` and change it back after you run second instance.
