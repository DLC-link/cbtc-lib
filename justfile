# List available recipes
default:
    @just --list

# --- Environment management ---

# Switch environment and credentials (generates .env)
env target credential:
    nu env.nu {{target}} {{credential}}

# List available environments and credential profiles
envs:
    nu env.nu

# --- Build & test ---

# Build the library
build:
    cargo build

# Run all tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy

# --- Auth guards (hidden) ---

[private]
require-password-grant:
    @grep -q '^KEYCLOAK_USERNAME=' .env 2>/dev/null || { echo "Error: this example requires a password-grant profile (KEYCLOAK_USERNAME)."; echo "Run: just env <environment> <credential>"; echo "Password-grant profiles: reader, incentive-sender, trader, ibtc-validator"; exit 1; }

[private]
require-client-credentials:
    @grep -q '^KEYCLOAK_CLIENT_SECRET=' .env 2>/dev/null || { echo "Error: this example requires a client-credentials profile (KEYCLOAK_CLIENT_SECRET)."; echo "Run: just env <environment> <credential>"; echo "Client-credentials profiles: network-reader, attestor"; exit 1; }

# --- Examples ---

# Run any example by name
run example:
    cargo run --example {{example}}

# List deposit addresses (password grant)
list-deposits: require-password-grant
    cargo run --example list_deposit_addresses

# List deposit addresses (client credentials), optionally filter by owner
list-deposits-cc filter="": require-client-credentials
    #!/usr/bin/env bash
    if [ -n "{{filter}}" ]; then export OWNER_FILTER="{{filter}}"; fi
    cargo run --example list_deposit_addresses_client_credential

# Check cBTC balance
check-balance: require-password-grant
    cargo run --example check_balance

# Mint cBTC flow
mint: require-password-grant
    cargo run --example mint_cbtc_flow

# Redeem cBTC flow
redeem: require-password-grant
    cargo run --example redeem_cbtc_flow

# Send cBTC
send: require-password-grant
    cargo run --example send_cbtc

# Accept incoming transfers
accept: require-password-grant
    cargo run --example accept_transfers

# Batch distribute cBTC
batch-distribute: require-password-grant
    cargo run --example batch_distribute

# Batch distribute with callback
batch-callback: require-password-grant
    cargo run --example batch_with_callback

# Consolidate UTXOs
consolidate: require-password-grant
    cargo run --example consolidate_utxos

# List incoming transfer offers
incoming: require-password-grant
    cargo run --example list_incoming_offers

# List outgoing transfer offers
outgoing: require-password-grant
    cargo run --example list_outgoing_offers

# Cancel pending offers
cancel: require-password-grant
    cargo run --example cancel_offers

# Check withdraw requests
withdrawals: require-password-grant
    cargo run --example check_withdraw_requests

# List active contracts
contracts:
    cargo run --example list_contracts

# Delete executed transfers
delete-transfers:
    cargo run --example delete_executed_transfers

# Stream cBTC events
stream: require-password-grant
    cargo run --example stream_cbtc

# Test burn cBTC
test-burn: require-password-grant
    cargo run --example test_burn_cbtc
