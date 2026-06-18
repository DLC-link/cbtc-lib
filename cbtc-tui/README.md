# cbtc-tui

Interactive terminal UI for operating CBTC on Canton, built on the `cbtc` library.
First release covers **configuration and read-only queries** (balance, incoming/
outgoing offers, deposit addresses, withdraw accounts/requests, DAR versions,
credentials).

## Run

```bash
cargo run -p cbtc-tui
```

## Config

Config lives at `~/.config/cbtc-tui/config.toml` (override with `--config` or
`$CBTC_TUI_CONFIG`), `chmod 0600`. A profile is a Canton **user** (keycloak login);
parties you can act/read as are fetched from Canton and switched in-app (`p`).
Built-in environment defaults exist for devnet/testnet/mainnet.

```toml
default_profile = "devnet-alice"

[[profiles]]
name               = "devnet-alice"
environment        = "devnet"
ledger_host        = "https://participant.example.com"
keycloak_host      = "https://keycloak.example.com"
keycloak_realm     = "..."
keycloak_client_id = "..."
keycloak_username  = "alice"
keycloak_password  = "..."
```

## Keys

`↑/↓` select · `Enter` activate/run · `p` switch party · `P` profiles · `r` refresh · `q` quit

## Logs

Rotating logs at `~/.local/state/cbtc-tui/cbtc-tui.log` (the TUI owns the terminal,
so nothing prints to stdout). Set `RUST_LOG=cbtc_tui=debug` for more detail.
