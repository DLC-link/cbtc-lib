# Instrument ID Management

Responsible: Max Webster-Dowsing
Created: February 10, 2026 4:31 PM
Created By: Max Webster-Dowsing
Last Edited: February 11, 2026 10:12 PM
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

**Audience:** App Developers building integrations that reference CBTC by its Canton Instrument ID.

</aside>

---

## What Are Instrument IDs?

Every token on Canton is identified by an **Instrument ID**, a combination of an **admin** party ID and a token **id** string, plus a **registry URL**. These values let any CIP-56-compliant tool discover and interact with CBTC.

---

## Why IDs Change

Instrument IDs can change due to network dynamics, including DAR upgrades, network migrations, or infrastructure changes. **Never hardcode Instrument IDs.** Fetch them dynamically from the metadata URL.

<aside>
⚠️

There is currently **no push notification** when Instrument IDs change. This is a known gap. Poll the metadata URL periodically.

</aside>

---

## Current IDs by Network

### Devnet

- **Registry URL:** [`https://api.utilities.digitalasset-dev.com`](https://api.utilities.digitalasset-dev.com)
- **Coordinator URL:** [`https://devnet.dlc.link/attestor-2`](https://devnet.dlc.link/attestor-2)
- **Metadata:** [View](https://api.utilities.digitalasset-dev.com/api/token-standard/v0/registrars/cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff/registry/metadata/v1/instruments)

```json
{
  "instrument_id": {
    "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
    "id": "CBTC"
  },
  "registry_url": "https://api.utilities.digitalasset-dev.com"
}
```

### Testnet

- **Registry URL:** [`https://api.utilities.digitalasset-staging.com`](https://api.utilities.digitalasset-staging.com)
- **Coordinator URL:** [`https://testnet.dlc.link/attestor-1`](https://testnet.dlc.link/attestor-1)
- **Metadata:** [View](https://api.utilities.digitalasset-staging.com/api/token-standard/v0/registrars/cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f/registry/metadata/v1/instruments)

```json
{
  "instrument_id": {
    "admin": "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f",
    "id": "CBTC"
  },
  "registry_url": "https://api.utilities.digitalasset-staging.com"
}
```

### Mainnet

- **Registry URL:** [`https://api.utilities.digitalasset.com`](https://api.utilities.digitalasset.com)
- **Coordinator URL:** [`https://mainnet.dlc.link/attestor-1`](https://mainnet.dlc.link/attestor-1)
- **Metadata:** [View](https://api.utilities.digitalasset.com/api/token-standard/v0/registrars/cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262/registry/metadata/v1/instruments)

```json
{
  "instrument_id": {
    "admin": "cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262",
    "id": "CBTC"
  },
  "registry_url": "https://api.utilities.digitalasset.com"
}
```

---

## Recommended Polling Pattern

```rust
use std::time::Duration;

// Poll every 5 minutes in production
const POLL_INTERVAL: Duration = Duration::from_secs(300);

async fn refresh_instrument_id(registry_url: &str, admin: &str) -> Result<InstrumentId> {
    let url = format!(
        "{}/api/token-standard/v0/registrars/{}/registry/metadata/v1/instruments",
        registry_url, admin
    );
    let response = reqwest::get(&url).await?.json::<InstrumentMetadata>().await?;
    Ok(response.instrument_id)
}
```

**Best practices:**

- Cache the Instrument ID locally and refresh on a schedule (every 5–15 minutes)
- Log a warning if the ID changes between polls, as this may indicate a DAR upgrade
- On startup, always fetch fresh rather than relying on cached values
- Handle fetch failures gracefully and use the last known good value

---

## Token Standard API Reference

Full documentation for the Canton Token Standard API: [Canton Token Standard Docs](https://docs.dev.sync.global/app_dev/token_standard/index.html#api-references)

**Requirements:** CBTC is CIP-56 compliant. No special requirements for holding CBTC.

---

<aside>
🔴

**⚙️ Engineering Review Required.** Instrument IDs and coordinator URLs must be verified against live deployments by Engineering (Jesse or Robert) before publication.

</aside>