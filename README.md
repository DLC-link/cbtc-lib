# Canton CBTC Token Library

A Rust library for interacting with the Canton blockchain to manage CBTC tokens using the Canton Token Standard, version 1 (CIP-0056) and version 2 (CIP-0112).

## Features

- **Send CBTC** - Transfer tokens to other parties
- **Accept CBTC** - Accept incoming transfers as a receiver
- **Mint CBTC** - Deposit BTC and mint CBTC tokens via the Bitsafe API
- **Redeem CBTC** - Burn CBTC tokens and withdraw BTC
- **Batch Distribution** - Efficiently distribute tokens to multiple recipients
- **UTXO Management** - Consolidate and split holdings
- **High-Volume Transfers** - Optimized for high-volume transfer operations
- **Multi-Environment** - Support for devnet, testnet, and mainnet
- **Token Standard Compliant** - Implements version 1 of the Canton Token Standard (CIP-0056) and version 2 (CIP-0112). Eight of the thirteen operations carry a V2 counterpart, and a caller names the version once on `cbtc::TokenClient`
- **Interactive TUI** - A terminal UI (`cbtc-tui`) to browse balances, offers, and accounts and submit commands without writing code

> **Important Setup Requirements**:
> - **For Send/Receive Operations**: The [Digital Asset Registry Utility](https://docs.digitalasset.com/utilities/mainnet/index.html) must be installed on your validator node
> - **For Mint/Redeem Operations**: Review the [CBTC Minting App Installation and User Guide](https://hub.bitsafe.finance/cbtc-minting-app-installation-and-user-guide) to install the required DAR files and configure permissions correctly

---

## Table of Contents

1. [Quick Start - How to Use This Library](#quick-start---how-to-use-this-library)
2. [Interactive TUI (cbtc-tui)](#interactive-tui-cbtc-tui)
3. [Installation](#installation)
4. [Configuration](#configuration)
5. [Usage Examples](#usage-examples)
   - [Core Operations](#core-operations)
   - [Key Concepts](#key-concepts)
6. [DAR Version Check](#dar-version-check)
7. [CBTC Mint & Redeem](#cbtc-mint--redeem)
   - [Minting CBTC (BTC → CBTC)](#minting-cbtc-btc--cbtc)
   - [Redeeming CBTC (CBTC → BTC)](#redeeming-cbtc-cbtc--btc)
   - [Understanding UTXO Management](#understanding-utxo-management)
8. [High-Volume Operations](#high-volume-operations)
9. [API Reference](#api-reference)
10. [Direct Canton API Usage (Reference)](#direct-canton-api-usage-reference)
11. [Testing](#testing)
12. [Contributing](#contributing)

---

## Quick Start - How to Use This Library

This library provides a high-level Rust interface for interacting with CBTC (Canton Bitcoin) tokens on the Canton blockchain. Here's how to get started:

### Prerequisites

Before using this library, you need:

1. **A Canton Participant Node** - Access to a Canton participant node (devnet, testnet, or mainnet)
2. **DA Registry Utility** - For sending and receiving CBTC tokens, the [Digital Asset Registry Utility](https://docs.digitalasset.com/utilities/mainnet/index.html) must be installed on your validator node
3. **Keycloak Credentials** - Authentication credentials for your participant node
4. **A Party ID** - Your unique party identifier on the Canton network
5. **CBTC Holdings** - Some CBTC tokens in your account (for sending/distributing)

### Three Ways to Use This Library

#### 1. Run the Examples (Fastest Way to Start)

The quickest way to see the library in action:

```bash
# Clone the repository
git clone <your-repo-url>
cd cbtc-lib

# Set up your environment
cp .env.example .env
# Edit .env with your Canton credentials

# Run an example
cargo run --example check_balance
cargo run --example send_cbtc
```

See [Quick Start with Examples](#quick-start-with-examples) for more details.

#### 2. Use as a Library in Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
# cbtc re-exports DamlDecimal, InstrumentId, Transfer, Meta, Account, Network
# and CBTC_TICKER, and the parameter types as `cbtc::types`, so a consumer
# needs no canton-lib dependency for the Token Standard types. If you add
# one, pin the same revision: a different pin makes Cargo build two `common`
# packages, and then cbtc::DamlDecimal and common::decimal::DamlDecimal differ.
cbtc = { git = "ssh://git@github.com/DLC-link/cbtc-lib", tag = "v0.7.0" }
keycloak = { git = "ssh://git@github.com/DLC-link/canton-lib", tag = "v0.8.0" }
```

`cbtc` pins `canton-lib` at `v0.8.0`, so the `keycloak` pin above names that
same tag. Pinning a different tag or a revision here builds two `common`
packages.
`cbtc-lib` is released as `v0.7.0`.

Or for local development:

```toml
[dependencies]
cbtc = { path = "path/to/cbtc-lib" }
```

Then in your code:

```rust
use cbtc::transfer;
use keycloak::login;

// Authenticate
let auth = login::password(login::PasswordParams {
    client_id: "your-client-id".to_string(),
    username: "your-username".to_string(),
    password: "your-password".to_string(),
    url: login::token_url("https://your-keycloak-host", "your-realm"),
}).await?;

// Send CBTC
transfer::submit(transfer::Params {
    // ... see Usage Examples section
}).await?;
```

#### 3. Understand the Low-Level API

For advanced users who want direct control, see [Direct Canton API Usage](#direct-canton-api-usage-reference) to learn how to interact with Canton's REST APIs directly.

### Common Operations

| Task              | Function                                     | Section                                                 |
| ----------------- | -------------------------------------------- | ------------------------------------------------------- |
| Check balance     | `cbtc::active_contracts::get()`              | [Understanding UTXO Management](#understanding-utxo-management) |
| Check DARs        | `cbtc::dar_check::check()`                   | [DAR Version Check](#dar-version-check)     |
| Send tokens       | `cbtc::transfer::submit()`                   | [Core Operations](#core-operations)                     |
| Accept tokens     | `cbtc::accept::submit()`                     | [Core Operations](#core-operations)                     |
| Batch send        | `cbtc::batch::submit_from_csv()`             | [High-Volume Operations](#high-volume-operations)       |
| Consolidate UTXOs | `cbtc::consolidate::check_and_consolidate()` | [Understanding UTXO Management](#understanding-utxo-management) |

---

## Interactive TUI (cbtc-tui)

`cbtc-tui` is an interactive terminal UI built on this library. It lets you manage Canton login profiles, switch parties and environments, browse on-ledger state, and submit commands — the same operations as the example scripts, without writing code.

### Launch

```bash
# with just (recommended)
just tui

# or directly
cargo run -p cbtc-tui
```

The app opens on the **Profiles** screen. Select a profile and press `Enter` to log in.

### Profiles & configuration

Config lives at `~/.config/cbtc-tui/config.toml` (created `0600`). Secrets are stored in plaintext, so keep the file private. The `devnet`, `testnet`, and `mainnet` environments are built in, so a profile only needs your ledger host and Keycloak credentials.

The fastest way to create a profile is to import one of the repo's `.env` files:

```bash
# create/update a profile named after the file's ENVIRONMENT (e.g. "mainnet")
just tui-import .env.mainnet

# or directly, with options
cargo run -p cbtc-tui -- --import-env .env.mainnet --profile-name mainnet --set-default
```

Importing merges into any existing config (replacing a same-named profile), writes an environment override only if the `.env` file carries one, and never prints the password.

### What you can do

**Queries (read-only):** Check Balance, Incoming Offers, Outgoing Offers, Deposit Addresses, Withdraw Accounts, Withdraw Requests, DAR Versions. Results are cached per party; press `Enter` on a result row for a detail popup.

**Commands:** accept / reject incoming offers, cancel outgoing offers, cancel-all-expired (batched), merge holdings, create a deposit account, create a withdraw account, and submit a withdraw. Every submission asks for confirmation first — with a red banner on **mainnet**.

### Keys

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move selection |
| `Tab` | Switch between the query list and results panes |
| `Enter` | Run the selected query / open row detail / confirm |
| `a` | Actions menu for the selected query (submit commands) |
| `p` | Switch party |
| `P` | Switch profile |
| `Esc` | Close popup / cancel |
| `q` | Quit |

Logs are written to `~/.local/state/cbtc-tui/cbtc-tui.log`.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cbtc = { git = "ssh://git@github.com/DLC-link/cbtc-lib", tag = "v0.7.0" }
```

`cbtc-lib` is released as `v0.7.0`.

Or for local development:

```toml
[dependencies]
cbtc = { path = "path/to/cbtc-lib" }
```

---

## Configuration

### Setup

1. **Create environment configuration**

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
```

2. **Configure your environment variables**

[`.env.example`](.env.example) lists every variable, with a comment on each
one.

You supply three things. Your participant node's JSON ledger API host goes in
`LEDGER_HOST`, and the template shows the API path it needs. Your party goes in
`PARTY_ID`. `ENVIRONMENT` names the network, and it takes `devnet`, `testnet`
or `mainnet`.

`ENVIRONMENT` supplies the decentralized party ID, the registry URL and the
Bitsafe API URL. The next section lists those values. Override any of the three
for a local or custom network, and an explicit value wins.

`cbtc-tui --import-env` writes no environment override from a fresh
`.env.example`, because the template comments all three overrides out. A user
who needs one uncomments the single variable that moved, then re-imports.

### Environment-Specific Values

#### Devnet

`ENVIRONMENT=devnet` selects these values, and `cbtc::Network::Devnet` returns
them to a Rust caller.

```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff
REGISTRY_URL=https://api.utilities.digitalasset-dev.com
BITSAFE_API_URL=https://api.devnet.bitsafe.finance
```

#### Testnet

`ENVIRONMENT=testnet` selects these values, and `cbtc::Network::Testnet`
returns them to a Rust caller.

```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f
REGISTRY_URL=https://api.utilities.digitalasset-staging.com
BITSAFE_API_URL=https://api.testnet.bitsafe.finance
```

#### Mainnet

`ENVIRONMENT=mainnet` selects these values, and `cbtc::Network::Mainnet`
returns them to a Rust caller.

```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262
REGISTRY_URL=https://api.utilities.digitalasset.com
BITSAFE_API_URL=https://api.mainnet.bitsafe.finance
```

---

## Quick Start with Examples

For quick experimentation, this library includes ready-to-run example programs. See the [`examples`](examples/README.md) directory for:

- `check_balance` - Check your CBTC balance and UTXO count
- `send_cbtc` - Send tokens to another party
- `accept_transfers` - Accept all pending incoming transfers
- `consolidate_utxos` - Consolidate multiple UTXOs
- `batch_distribute` - Distribute tokens to multiple recipients from a CSV file

Run examples from the project root:

```bash
cargo run --example check_balance
```

See the [examples README](examples/README.md) for detailed instructions.

---

## Usage Examples

This library provides several high-level operations for working with CBTC tokens. Below is a quick reference - for complete working examples, see the [`examples`](examples) directory.

### Core Operations

| Operation                | Example File                                                       | Run Command                                  | Description                                      |
| ------------------------ | ------------------------------------------------------------------ | -------------------------------------------- | ------------------------------------------------ |
| **Mint CBTC**            | [`mint_cbtc_flow.rs`](examples/mint_cbtc_flow.rs)                  | `cargo run --example mint_cbtc_flow`         | Set up deposit account for BTC deposits          |
| **Redeem CBTC**          | [`redeem_cbtc_flow.rs`](examples/redeem_cbtc_flow.rs)              | `cargo run --example redeem_cbtc_flow`       | Burn CBTC and withdraw BTC                       |
| **Check Balance**        | [`check_balance.rs`](examples/check_balance.rs)                    | `cargo run --example check_balance`          | View your CBTC balance and UTXO count            |
| **Send CBTC**            | [`send_cbtc.rs`](examples/send_cbtc.rs)                            | `cargo run --example send_cbtc`              | Transfer tokens to another party                 |
| **Accept CBTC**          | [`accept_transfers.rs`](examples/accept_transfers.rs)              | `cargo run --example accept_transfers`       | Accept incoming transfers                        |
| **List Incoming Offers** | [`list_incoming_offers.rs`](examples/list_incoming_offers.rs)      | `cargo run --example list_incoming_offers`   | List pending transfers where you're the receiver |
| **List Outgoing Offers** | [`list_outgoing_offers.rs`](examples/list_outgoing_offers.rs)      | `cargo run --example list_outgoing_offers`   | List pending transfers where you're the sender   |
| **Check DARs**           | [`check_dars.rs`](examples/check_dars.rs)                          | `cargo run --example check_dars`             | Verify required DAR packages are uploaded        |
| **Cancel Offers**        | [`cancel_offers.rs`](examples/cancel_offers.rs)                    | `cargo run --example cancel_offers`          | Cancel all pending outgoing transfers            |
| **Stream CBTC**          | [`stream.rs`](examples/stream.rs)                                  | `cargo run --example stream_cbtc`            | Stream CBTC to a single receiver multiple times  |
| **Batch Distribution**   | [`batch_distribute.rs`](examples/batch_distribute.rs)              | `cargo run --example batch_distribute`       | Distribute to multiple recipients from CSV       |
| **Consolidate UTXOs**    | [`consolidate_utxos.rs`](examples/consolidate_utxos.rs)            | `cargo run --example consolidate_utxos`      | Merge multiple UTXOs into one                    |

### Key Concepts

**Authentication**: All operations require Keycloak/OIDC authentication. The library handles token management - you just provide credentials.

**UTXO Model**: CBTC uses a UTXO (Unspent Transaction Output) model similar to Bitcoin. Each holding is a separate UTXO that can be split or combined.

**Two-Phase Transfers**:

1. Sender creates a transfer offer
2. Receiver must accept the transfer to complete it

**BTC/CBTC Bridge**: The mint/redeem flow allows you to bridge between native Bitcoin and CBTC tokens:

- **Minting**: Create deposit account → Get BTC address → Send BTC → Attestor network confirms (6+ blocks) → CBTC automatically minted
- **Redeeming**: Burn CBTC → Create withdraw request → Attestor network sends BTC

See the [examples README](examples/README.md) for detailed usage instructions.

---

## DAR Version Check

Before using cbtc-lib, your Canton participant node must have all required DAR packages uploaded. The DAR check tool verifies this by scanning the DAR files in `cbtc-dars/dars/` and comparing the expected packages against what is uploaded on your participant.

### Running the Check

```bash
cargo run --example check_dars
```

The tool authenticates via Keycloak, scans the DAR files in `cbtc-dars/dars/` to determine expected packages, fetches the list of packages from your participant's Ledger API (`GET /v2/packages`), and compares them. It exits with code 0 if all packages are present, or code 1 if any are missing.

**Required environment variables**: `KEYCLOAK_HOST`, `KEYCLOAK_REALM`, `KEYCLOAK_CLIENT_ID`, `KEYCLOAK_USERNAME`, `KEYCLOAK_PASSWORD`, `LEDGER_HOST`

> **Note**: This repo may include DAR versions ahead of what is currently deployed on Canton Network mainnet. If the check reports missing packages, they may not yet be required for your environment. To verify which versions are required:
> - **Splice DARs**: [hyperledger-labs/splice daml/dars](https://github.com/hyperledger-labs/splice/tree/main/daml/dars) (select the tag matching your environment release)
> - **Utility DARs**: [Canton Network Utility releases](https://docs.digitalasset.com/utilities/releases/index.html)

### Using in Your Code

```rust
let result = cbtc::dar_check::check(cbtc::dar_check::Params {
    ledger_host: "https://participant.example.com".to_string(),
    access_token: auth.access_token,
    dar_dirs: vec![
        "cbtc-dars/dars/dependencies".to_string(),
        "cbtc-dars/dars/cbtc".to_string(),
    ],
}).await?;

if result.status == cbtc::dar_check::DarCheckStatus::Fail {
    for info in &result.missing {
        println!("Missing: {} v{}", info.name, info.version);
    }
}
```

---

## CBTC Mint & Redeem

> **Important**: Before using mint/redeem operations, please review the [CBTC Minting App Installation and User Guide](https://hub.bitsafe.finance/cbtc-minting-app-installation-and-user-guide). This guide covers the required DAR file installation and permission configuration needed on your Canton participant node for mint/redeem functionality to work properly.

### Overview

The mint/redeem functionality allows you to bridge between native Bitcoin and CBTC tokens on the Canton network. This is powered by the **Bitsafe Attestor Network**, a decentralized network that monitors Bitcoin transactions and confirms deposits/withdrawals.

### Minting CBTC (BTC → CBTC)

**Flow**:

1. Create a deposit account with the `mint_redeem` module
2. Receive a unique BTC deposit address from the attestor
3. Send BTC to that address (external to this library)
4. Attestor network monitors and confirms your deposit (6+ blocks)
5. Attestors automatically mint CBTC tokens to your party

**Example**:

```bash
# Run the mint flow example to set up deposit account
cargo run --example mint_cbtc_flow

# After sending BTC, monitor for minted CBTC by checking your balance
cargo run --example check_balance
```

See [mint_cbtc_flow.rs](examples/mint_cbtc_flow.rs) for complete code.

**Note**: This library does NOT monitor Bitcoin transactions. The attestor network handles all monitoring and automatically mints CBTC after confirmation. You can periodically run `check_balance` to see when CBTC has been minted to your account.

### Redeeming CBTC (CBTC → BTC)

**Flow**:

1. Create a withdraw account with your destination BTC address
2. Burn CBTC tokens to create a withdraw request
3. Attestor network processes the request
4. Receive BTC at your destination address

**Example**:

```bash
# Run the redeem flow example
cargo run --example redeem_cbtc_flow

# Test burning CBTC with an existing withdraw account
cargo run --example test_burn_cbtc
```

See [redeem_cbtc_flow.rs](examples/redeem_cbtc_flow.rs) for complete code.

### Required Configuration

Mint and redeem need a Bitsafe API URL, and `ENVIRONMENT` supplies it.
`BITSAFE_API_URL` is an override for a local or custom network:

```bash
BITSAFE_API_URL=https://api.devnet.bitsafe.finance  # or api.testnet.bitsafe.finance / api.mainnet.bitsafe.finance
```

### Understanding UTXO Management

**What are UTXOs?**

Every CBTC holding is a UTXO (Unspent Transaction Output), similar to Bitcoin. Each transfer can create new UTXOs, and over time you may accumulate many small ones.

**Why Consolidate?**

- **Performance**: Canton has a soft limit of **10 UTXOs per party** per token type
- **Node Efficiency**: Fewer UTXOs reduce database and memory usage
- **Network Load**: Smaller transactions with fewer inputs

**Best Practice**: Consolidate regularly, especially for high-volume operations. See [consolidate_utxos.rs](examples/consolidate_utxos.rs) for example code.

---

## High-Volume Operations

For applications running high-volume CBTC transfers (e.g., payment processors, exchanges, automated trading):

### Best Practices

1. **Monitor UTXOs**: Consolidate when approaching 10 UTXOs per party
2. **Use Batch Operations**: `batch::submit_from_csv()` for efficient multi-recipient transfers
3. **Consolidate Proactively**: Check and consolidate before large distributions
4. **Handle Both Parties**: If you control sender and receiver, consolidate both

### Recommended Workflow

```bash
# 1. Check UTXO count and consolidate if needed
cargo run --example consolidate_utxos

# 2. Run batch distribution
cargo run --example batch_distribute
```

See [batch_distribute.rs](examples/batch_distribute.rs) and [batch_with_callback.rs](examples/batch_with_callback.rs) for complete examples with callbacks and logging.

---

## API Reference

Every module below is a re-export of `canton-lib`'s `token` crate. `cbtc`
adds `mint_redeem` and nothing else.

Eight of the thirteen operations carry a Token Standard V2 counterpart in a
`v2` submodule, for example `cbtc::transfer::v2::submit` beside
`cbtc::transfer::submit`.
Alternatively `cbtc::TokenClient` takes the version in its config and
applies it to every write method and to `holdings`, `balance` and
`utxo_count`, so a caller names the version once. `incoming_offers` and
`outgoing_offers` read the same contracts under either version.

`active_contracts` reaches V2 through its `account` field. `credentials` and
`dar_check` never call the token registry, so no version applies to them, and
`utils` reads both versions with one implementation. `allocation` is the one
gap: it has a V1 equivalent and V2 defines a form for it, and `canton-lib`
supplies neither the route nor the account-shaped leg. [#81][alloc-v2] tracks
it.

[alloc-v2]: https://github.com/DLC-link/cbtc-lib/issues/81

The library supplies no default ticker. Where an operation needs an
instrument, it takes one from the caller, because Bitsafe plans to support
instruments other than CBTC. `cbtc::CBTC_TICKER` names the CBTC ticker for a
caller that wants it. Naming the value is not defaulting to it: every
operation still reads its instrument from its caller. An operation that acts
on one named contract needs none, as `accept::submit` and `reject::submit`
show, and `credentials` and `dar_check` name no instrument either.
`mint_redeem::list_holdings` takes one and filters on it, which closed issue
#74's defect.
`cbtc` re-exports `DamlDecimal`, `InstrumentId`, `Transfer`, `Meta`,
`Account`, `Network` and `CBTC_TICKER` at its root, and the parameter types
those signatures name as `cbtc::types`, so a consumer needs no `canton-lib`
dependency to name them.
`transfer::Params.transfer` and `transfer::v2::Params.transfer` are
`cbtc::Transfer` and `cbtc::types::v2::Transfer`. `cbtc` does not re-export
`common` whole. The reads — `active_contracts::get`,
`utils::fetch_incoming_transfers`, `utils::fetch_outgoing_transfers` and
`TokenClient::holdings` — return `Vec<JsActiveContract>`, which comes from
the crates.io crate `canton-api-client`, not from `canton-lib`.

### Core Modules

#### `cbtc::transfer`

- `submit(Params)` - Send CBTC to a single recipient

#### `cbtc::accept`

- `submit(Params)` - Accept an incoming CBTC transfer

#### `cbtc::cancel_offers`

- `withdraw_all(WithdrawAllParams)` - Withdraw all pending outgoing transfers
- `submit(Params)` - Withdraw a specific transfer offer

#### `cbtc::distribute`

- `submit(Params)` - Distribute CBTC to multiple recipients

#### `cbtc::batch`

- `submit_from_csv(Params)` - Batch distribution from CSV file

#### `cbtc::consolidate`

- `check_and_consolidate(CheckConsolidateParams)` - Check and consolidate if needed
- `get_utxo_count(GetUtxoCountParams)` - Get UTXO count
- `consolidate_utxos(ConsolidateParams)` - Force consolidation

#### `cbtc::split`

- `submit(Params)` - Split holdings into specific amounts

#### `cbtc::dar_check`

- `check(Params)` - Verify all required DAR packages are uploaded to the participant

#### `cbtc::active_contracts`

- `get(Params)` - Get active CBTC holdings

#### `mint_redeem::mint`

- `list_deposit_accounts(Params)` - Get all deposit accounts for your party
- `create_deposit_account(Params)` - Create new deposit account for receiving BTC
- `get_bitcoin_address(Params)` - Get Bitcoin address for a deposit account
- `get_deposit_account_status(Params)` - Get full status including Bitcoin address and last processed block

#### `mint_redeem::redeem`

- `list_withdraw_accounts(Params)` - Get all withdraw accounts
- `create_withdraw_account(Params)` - Create withdraw account with BTC destination
- `list_holdings(ListHoldingsParams)` - Get holdings of one instrument, for
  burning. `instrument_id` is required: the library supplies no default
  ticker. `cbtc::CBTC_TICKER` names the CBTC value, and naming it is not
  defaulting to it.
- `submit_withdraw(SubmitWithdrawParams)` - Burn CBTC and request BTC withdrawal
- `list_withdraw_requests(Params)` - Monitor withdrawal status

### Helper Modules

#### `keycloak::login`

- `password(PasswordParams)` - Authenticate with username/password
- `client_credentials(ClientCredentialsParams)` - Service account authentication

`KeycloakConfig.url` takes `keycloak::login::token_url`, not the deprecated
`password_url`. The doc comment on `KeycloakConfig` recommends `password_url`;
[canton-lib#54](https://github.com/DLC-link/canton-lib/issues/54) tracks
fixing it at its source.

#### `ledger`

- Low-level ledger API operations
- WebSocket streaming for real-time updates

---

## Direct Canton API Usage (Reference)

For teams who want to understand the underlying protocol or implement custom workflows, here's how to interact with Canton APIs directly.

### Prerequisites

- Access to a Canton participant node
- Valid OIDC authentication token
- Understanding of Canton's UTXO model

### Get Active Contracts

```bash
LEDGER_OFFSET=$(curl -X GET "$LEDGER_HOST/v2/state/ledger-end" \
  -H "Authorization: Bearer $ACCESS_TOKEN" | jq -r '.offset')

curl -X POST $LEDGER_HOST/v2/state/active-contracts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{
    "filter": {
      "filtersByParty": {
        "$SENDER_PARTY_ID": {
          "cumulative": [{
            "identifierFilter": {
              "InterfaceFilter": {
                "value": {
                  "interfaceId": "#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding",
                  "includeInterfaceView": true,
                  "includeCreatedEventBlob": true
                }
              }
            }
          }]
        }
      }
    },
    "verbose": false,
    "activeAtOffset": '$(echo $LEDGER_OFFSET | jq -R 'tonumber')'
  }' | jq
```

### Get Factory Disclosures

```bash
curl -X POST $REGISTRY_URL/api/token-standard/v0/registrars/$DECENTRALIZED_PARTY_ID/registry/transfer-instruction/v1/transfer-factory \
  -H "Content-Type: application/json" \
  -d '{
    "choiceArguments": {
      "expectedAdmin": "'$DECENTRALIZED_PARTY_ID'",
      "transfer": {
        "sender": "'$SENDER_PARTY_ID'",
        "receiver": "'$RECEIVER_PARTY_ID'",
        "amount": 0.5,
        "instrumentId": {
          "admin": "'$DECENTRALIZED_PARTY_ID'",
          "id": "CBTC"
        },
        "requestedAt": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'",
        "executeBefore": "'$(date -u -d "+1 days" +"%Y-%m-%dT%H:%M:%SZ")'",
        "inputHoldingCids": ["'$HOLDING_CID'"]
      },
      "extraArgs": {
        "context": {"values": {}},
        "meta": {"values": {}}
      }
    },
    "excludeDebugFields": true
  }' | jq
```

### Submit Transfer

See [example_transfer.sh](example_transfer.sh) for a complete example.

### Accept Transfer

```bash
# Get accept context
curl -X POST $REGISTRY_URL/api/token-standard/v0/registrars/$DECENTRALIZED_PARTY_ID/registry/transfer-instruction/v1/$TRANSFER_OFFER_CID/choice-contexts/accept \
  -H "Content-Type: application/json" \
  -d '{"meta":{}}' | jq

# Submit acceptance (use disclosed contracts from above)
curl -X POST $LEDGER_HOST/v2/commands/submit-and-wait-for-transaction \
  -H "Authorization: Bearer $RECEIVER_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": {
      "commands": [{
        "ExerciseCommand": {
          "templateId": "#splice-api-token-transfer-instruction-v1:Splice.Api.Token.TransferInstructionV1:TransferInstruction",
          "contractId": "'$TRANSFER_OFFER_CID'",
          "choice": "TransferInstruction_Accept",
          "choiceArgument": {
            "extraArgs": {
              "context": {"values": '$CHOICE_CONTEXT_VALUES'},
              "meta": {"values": {}}
            }
          }
        }
      }],
      "commandId": "'$(uuidgen)'",
      "actAs": ["'$RECEIVER_PARTY'"],
      "disclosedContracts": '$DISCLOSED_CONTRACTS'
    },
    "transactionFormat": {
      "transactionShape": "TRANSACTION_SHAPE_LEDGER_EFFECTS",
      "eventFormat": {
        "filtersByParty": {"'$RECEIVER_PARTY'": {}},
        "verbose": true
      }
    }
  }' | jq
```

---

## Contributing

We welcome contributions from the Canton ecosystem! This library is designed to help developers build on Canton's CBTC token standard.

### How to Contribute

#### Reporting Issues

Found a bug or have a feature request?

1. Check [existing issues](../../issues) to avoid duplicates
2. Open a new issue with:
   - Clear description of the problem or feature
   - Steps to reproduce (for bugs)
   - Expected vs actual behavior
   - Your environment (Canton network, Rust version, OS)

#### Contributing Code

1. **Fork the repository** and create a feature branch

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**

   - Follow Rust best practices and naming conventions
   - Keep library code free of environment variable dependencies
   - Add tests for new functionality (when applicable)
   - Update documentation and examples

3. **Test your changes**

   ```bash
   # Build the library
   cargo build --release

   # Build examples
   cargo build --examples --release

   # Run clippy for linting
   cargo clippy --all-targets --all-features

   # Format code
   cargo fmt --all
   ```

4. **Submit a pull request**
   - Provide a clear description of your changes
   - Reference any related issues
   - Ensure CI checks pass

### Development Guidelines

#### Code Organization

- **Library code** (`src/`) should:

  - Accept all configuration as function parameters (no `env::var()` calls)
  - Be environment-agnostic and testable
  - Follow dependency injection patterns

- **Example code** (`examples/`) can:
  - Read from environment variables
  - Demonstrate practical usage patterns
  - Show best practices for error handling

#### Testing

- **Integration tests** require live Canton network access and credentials
- Set required environment variables (see `.env.example`)
- Tests expect to connect to real endpoints - they will fail without proper setup
- This is intentional: tests validate real-world behavior

#### Documentation

- Add doc comments to public functions using `///`
- Include usage examples in doc comments
- Update README.md for significant changes
- Keep examples up-to-date with API changes

### Getting Help

- Review the [examples](examples) directory for usage patterns
- Check the [Canton documentation](https://docs.digitalasset.com/canton) for protocol details
- Open a discussion for questions about the library
- Join the Canton community for broader ecosystem questions

### License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Testing

This library includes integration tests that validate real-world interactions with Canton networks.

### Running Tests

`cargo test` needs no credentials and no network. It runs the unit tests only.

The live tests carry `#[ignore]`, so they run only with
`cargo test -- --ignored`. Those need a Canton participant node, valid
Keycloak credentials and network access. The hosts in `.env` do not resolve
today, so they fail against a stock checkout; #68 tracks repairing them.

Set up your environment:

```bash
cp .env.example .env
# Edit .env with your Canton credentials
```

The `--ignored` tests in `src/mint_redeem` take the Bitsafe API URL from
`ENVIRONMENT`, exactly as the examples do. `BITSAFE_API_URL` overrides it.

Run the `--ignored` tests one at a time:

```bash
cargo test -- --ignored --test-threads=1
```

Every live test calls `dotenvy::dotenv()`, which writes the process
environment. The harness runs tests in parallel, so one test can read a
variable while another writes it. The run then fails with
`invalid_grant: Invalid user credentials` although the credentials are
correct. Measured on 22 September 2026: three parallel runs passed 2, 3 and 3
of 6, and two runs with `--test-threads=1` both passed 4 of 6. #80 tracks the
fix.

Run tests:

```bash
# Build the library (always works)
cargo build --release

# Run integration tests (requires credentials)
cargo test --lib

# Note: Tests will fail without proper environment variables
# This is expected and intentional - they validate real network behavior
```

### Why Tests Require Credentials

Unlike unit tests, these are **integration tests** that:

- Connect to actual Canton participant nodes
- Perform real ledger operations
- Validate end-to-end workflows

This ensures the library works correctly with real Canton infrastructure, not just in isolation.

---

## License

MIT License - see [LICENSE](LICENSE) file for details

## Resources

- [Canton Token Standard V1 (CIP-0056)](https://github.com/canton-foundation/cips/blob/main/cip-0056/cip-0056.md)
- [Canton Token Standard V2 (CIP-0112)](https://github.com/canton-foundation/cips/blob/main/cip-0112/cip-0112.md)
- [Canton Coin Fee Removal (CIP-0078)](https://github.com/canton-foundation/cips/blob/main/cip-0078/cip-0078.md)
- [Canton Documentation](https://docs.digitalasset.com/canton)
- [Canton Network](https://www.canton.network/)
