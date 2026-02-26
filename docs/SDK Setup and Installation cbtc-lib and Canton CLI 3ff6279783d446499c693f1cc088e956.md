# SDK Setup and Installation: cbtc-lib and Canton CLI

Responsible: Max Webster-Dowsing
Created: February 10, 2026 6:52 PM
Created By: Max Webster-Dowsing
Last Edited: February 11, 2026 10:11 PM
Last Edited By: Max Webster-Dowsing
Pillars: CBTC (https://www.notion.so/CBTC-2c3636dd0ba580cf8739cd330148f78a?pvs=21)
Priority Level: High
Product: CBTC
Projects: CBTC Documentation Overhaul (https://www.notion.so/CBTC-Documentation-Overhaul-56ac9d22e884416f8f5db5bb7ead1d04?pvs=21)
Status: In Review
Status Update: Not started
Type: Guide

<aside>
👥

**Audience:** App Developers and Infrastructure Integrators setting up their local environment to build with CBTC on the Canton Network.

</aside>

<aside>
⚠️

**API Disclaimer.** CBTC APIs have no formal versioning policy today. All SDK interfaces described in this guide are **subject to change**. Breaking changes are communicated via #cbtc-ecosystem and the changelog. This disclaimer will be updated once a formal versioning and stability policy is established.

</aside>

---

This page is your single reference for installing and configuring everything you need to build with CBTC. If you've already completed setup, head straight to the **Quick Start** to mint your first wrapped Bitcoin.

---

## System Requirements

| Requirement | Details |
| --- | --- |
| **Rust toolchain** | Latest stable. Install via [rustup.rs](http://rustup.rs) |
| **Canton participant node** | Running and connected to devnet, testnet, or mainnet. See [Canton documentation](https://docs.digitalasset.com/canton) |
| **DA Registry Utility** | Installed and configured. See [Digital Asset Utilities docs](https://docs.digitalasset.com/utilities/mainnet/index.html) |
| **Keycloak credentials** | Username, password, client ID, and client secret for your environment |
| **Party ID** | Your Canton Party ID, obtained during onboarding |

---

## Install cbtc-lib (Rust)

`cbtc-lib` is BitSafe's primary SDK for CBTC operations: minting, burning, transferring, UTXO management, and balance queries. It wraps the Canton Ledger API with type-safe Rust functions.

- **Repository:** [github.com/DLC-link/cbtc-lib](http://github.com/DLC-link/cbtc-lib)
- **Current version:** v0.0.1
- **Licence:** *Check repository*

### Add to your project

Add `cbtc-lib` to your `Cargo.toml`:

```toml
[dependencies]
cbtc-lib = { git = "https://github.com/DLC-link/cbtc-lib.git", tag = "v0.0.1" }
```

<aside>
📌

**Pin your version.** Always reference a specific tag (e.g. `v0.0.1`) rather than `main`. The library is under active development and `main` may contain breaking changes between releases.

</aside>

### Key modules

| Module | Purpose |
| --- | --- |
| `mint_redeem::mint` | Create deposit accounts, get deposit addresses, trigger minting |
| `mint_redeem::redeem` | Burn CBTC and withdraw to a BTC address |
| `cbtc::transfer` | Create and accept two-phase CBTC transfers |
| `cbtc::active_contracts` | Query current CBTC holdings for a party |
| `cbtc::consolidate` | Merge multiple UTXO holdings into fewer contracts |
| `cbtc::split` | Split a single holding into multiple UTXOs |
| `cbtc::batch` | Batch operations for sending to multiple recipients |
| `cbtc::distribute` | Distribute CBTC across multiple parties |

---

## Install canton-lib

`canton-lib` is the lower-level library that `cbtc-lib` depends on. It handles Canton Ledger API communication, authentication, and Daml contract interactions. You typically won't use it directly unless you're building custom Canton integrations beyond CBTC.

- **Repository:** [github.com/DLC-link/canton-lib](http://github.com/DLC-link/canton-lib)

### Add to your project

```toml
[dependencies]
canton-lib = { git = "https://github.com/DLC-link/canton-lib.git", tag = "v0.0.1" }
```

The `keycloak` module in `canton-lib` provides the authentication helper used across all CBTC operations:

```rust
use keycloak::login;

let token = login(
    &keycloak_url,
    &client_id,
    &client_secret,
    &username,
    &password,
).await?;
```

---

## Install CBTC DAR Files

DAR (Daml Archive) files contain the smart contract templates that power CBTC on Canton. They must be installed on your participant node before you can interact with CBTC.

**Download:** [github.com/DLC-link/cbtc-lib/tree/main/cbtc-dars](http://github.com/DLC-link/cbtc-lib/tree/main/cbtc-dars)

Install the DAR files on your Canton participant node using the Canton console or your deployment tooling. The specific installation method depends on your Canton setup. Refer to the [Canton documentation](https://docs.digitalasset.com/canton) for details.

<aside>
💡

**DAR version and Instrument IDs are linked.** When DAR files are upgraded on the network, Instrument IDs may change. Always fetch Instrument IDs dynamically from the metadata endpoint rather than hardcoding them. See the **Instrument ID Management** page for the polling pattern.

</aside>

---

## Environment Configuration

Set these variables before running any CBTC commands or code. Values differ per environment.

| Variable | Devnet | Testnet | Mainnet |
| --- | --- | --- | --- |
| `REGISTRY_URL` | [`https://api.utilities.digitalasset-dev.com`](https://api.utilities.digitalasset-dev.com) | [`https://api.utilities.digitalasset-staging.com`](https://api.utilities.digitalasset-staging.com) | [`https://api.utilities.digitalasset.com`](https://api.utilities.digitalasset.com) |
| `ATTESTOR_URL` | [`https://attestor.bitsafe.dev`](https://attestor.bitsafe.dev) | [`https://attestor.bitsafe.testnet`](https://attestor.bitsafe.testnet) | [`https://attestor.bitsafe.com`](https://attestor.bitsafe.com) |
| `DECENTRALIZED_PARTY_ID` | *Provided during onboarding* | *Provided during onboarding* | *Provided during onboarding* |

### Example .env file

```bash
# Environment (choose one: devnet, testnet, mainnet)
REGISTRY_URL="https://api.utilities.digitalasset-staging.com"
ATTESTOR_URL="https://attestor.bitsafe.testnet"
DECENTRALIZED_PARTY_ID="your-party-id-here"

# Authentication (Keycloak)
KEYCLOAK_URL="your-keycloak-url"
KEYCLOAK_CLIENT_ID="your-client-id"
KEYCLOAK_CLIENT_SECRET="your-client-secret"
KEYCLOAK_USERNAME="your-username"
KEYCLOAK_PASSWORD="your-password"

# Canton participant
LEDGER_HOST="your-ledger-host"
LEDGER_PORT="your-ledger-port"
```

---

## Migration Guide: December 2025 Restructure

In December 2025, `cbtc-lib` underwent a major restructure led by Ferenc. If you were using an earlier version, you will need to update your imports and module paths.

### What changed

- Module hierarchy was reorganised for clarity
- Some function signatures were updated
- `canton-lib` was extracted as a separate dependency

### How to migrate

1. Update your `Cargo.toml` to pin to `v0.0.1` (the first post-restructure tag)
2. Update all `use` statements to match the new module paths (see the module table above)
3. Review the [cleanup PR](https://github.com/DLC-link/cbtc-lib/pull/11) for a full diff of changes

<aside>
⚠️

**Breaking change.** Code written against pre-restructure `cbtc-lib` will not compile against v0.0.1 without import updates. There are no runtime behaviour changes, only module paths and function signatures moved.

</aside>

---

## Verify Your Installation

Run this minimal check to confirm everything is wired up:

```rust
use keycloak::login;
use cbtc::active_contracts;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Authenticate
    let token = login(
        &std::env::var("KEYCLOAK_URL")?,
        &std::env::var("KEYCLOAK_CLIENT_ID")?,
        &std::env::var("KEYCLOAK_CLIENT_SECRET")?,
        &std::env::var("KEYCLOAK_USERNAME")?,
        &std::env::var("KEYCLOAK_PASSWORD")?,
    ).await?;

    println!("✅ Authenticated successfully");

    // 2. Query holdings (should return empty if no CBTC yet)
    let holdings = active_contracts::get_active_contracts(
        &ledger_client,
        &std::env::var("PARTY_ID")?,
        &token,
    ).await?;

    println!("✅ Connected to Canton. Current CBTC holdings: {}", holdings.len());
    Ok(())
}
```

If both checks pass, you're ready. Head to the **Quick Start** to mint your first CBTC.

---

## Next Steps

- **Quick Start:** Mint your first wrapped Bitcoin in 15 minutes
- **API Reference:** Full Canton Ledger API endpoint documentation
- **Instrument ID Management:** How to fetch and poll for the latest CBTC Instrument IDs
- **Authentication Guide:** Detailed Keycloak setup and Auth0 community example

---

<aside>
🔴

**Engineering review required before publication.** Module paths, function signatures, and the verification script must be validated by Jesse or Ferenc against the actual `cbtc-lib` v0.0.1 source. The DAR installation instructions should be reviewed by Robert to confirm the correct procedure for each Canton deployment model.

</aside>