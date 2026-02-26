# CBTC Testnet Guide: Sandbox Environment for Developers

Responsible: Max Webster-Dowsing
Created: February 10, 2026 4:31 PM
Created By: Max Webster-Dowsing
Last Edited: February 13, 2026 2:38 AM
Last Edited By: Kadeem Clarke
Pillars: CBTC (https://www.notion.so/CBTC-2c3636dd0ba580cf8739cd330148f78a?pvs=21)
Priority Level: High
Product: CBTC
Projects: CBTC Documentation Overhaul (https://www.notion.so/CBTC-Documentation-Overhaul-56ac9d22e884416f8f5db5bb7ead1d04?pvs=21)
Status: In Review
Status Update: Not started
Type: Guide

<aside>
👥

**Audience:** All developers. Start here to experiment with CBTC before going to production on mainnet.

</aside>

<aside>
⚠️

**API Disclaimer:** CBTC APIs are subject to change. Testnet behavior may differ from mainnet in some respects (see Parity section below).

</aside>

---

## Overview: CBTC Testnet for Developers

The CBTC testnet is a sandbox environment where developers can experiment with the full mint, burn, and transfer lifecycle using **test BTC** with no real funds at risk. We recommend all integrations start on testnet before deploying to mainnet.

---

## How to Get Testnet CBTC: Three Options

### Option 1: CBTC Testnet Faucet (Recommended)

The fastest way to get testnet tokens:

[**CBTC Testnet Faucet →**](https://cbtc-faucet.bitsafe.finance/)

Simply enter your testnet wallet address and receive testnet CBTC and CC (for gas) instantly.

<aside>
💡

**Quick and easy:** No setup required. Just enter your address and go.

</aside>

### Option 2: Bron Wallet Testnet Environment

If the faucet is unavailable or you want a full testnet wallet environment:

1. **Create a new workspace** in [Bron Wallet](https://bron.app)
2. Enable **Developer settings → Testnet mode** during workspace creation
3. **Create a testnet account** (toggle Testnet ON in account settings)
4. Select a **Trusted third party** (e.g., Qrypt) for key recovery
5. Use the faucet to fund your new testnet account

### Option 3: Mint via Testnet Flow

You can also mint testnet CBTC through the same flow as mainnet, using testnet BTC. This is useful for testing the full minting integration:

1. Set up your participant pointing at the **testnet** Canton network
2. Install CBTC DAR files
3. Follow the standard minting flow (see **Minting and Burning Guide**)
4. Use testnet BTC from a Bitcoin testnet faucet

---

## Testnet Environment Details

| Property | Testnet | Mainnet |
| --- | --- | --- |
| **Registry URL** | [`https://api.utilities.digitalasset-staging.com`](https://api.utilities.digitalasset-staging.com) | [`https://api.utilities.digitalasset.com`](https://api.utilities.digitalasset.com) |
| **Coordinator URL** | [`https://testnet.dlc.link/attestor-1`](https://testnet.dlc.link/attestor-1) | [`https://mainnet.dlc.link/attestor-1`](https://mainnet.dlc.link/attestor-1) |
| **Instrument ID (admin)** | `cbtc-network::12201b17...508f` | `cbtc-network::12205af3...b262` |
| **BTC network** | Bitcoin Testnet | Bitcoin Mainnet |
| **Faucet** | [cbtc-faucet.bitsafe.finance](http://cbtc-faucet.bitsafe.finance) | N/A (real BTC required) |

### Testnet Instrument ID (Full)

```json
{
    "instrument_id": {
        "admin": "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f",
        "id": "CBTC"
    },
    "registry_url": "https://api.utilities.digitalasset-staging.com"
}
```

---

## Testnet vs. Mainnet: What Is the Same and What Differs

<aside>
📋

Understanding what is the same and what differs between testnet and mainnet is critical for a smooth production launch.

</aside>

### ✅ What Is Identical

- **DAR files:** Same CBTC Daml packages
- **API surface:** Same Canton Ledger API endpoints and Daml template interfaces
- **Mint and burn flows:** Same step-by-step process
- **Governance model:** Same Attestor threshold approval mechanism
- **Token standard:** CIP-56 compliant on both networks

### ⚠️ What Differs

- **Attestor set:** Testnet runs a **smaller** Attestor set than mainnet
- **Confirmation times:** May be faster on testnet due to less Bitcoin network congestion
- **Instrument IDs:** Different across networks — always fetch from the metadata URL, never hardcode
- **BTC:** Testnet uses test BTC with no real value
- **Faucet availability:** Testnet has a faucet; mainnet requires real BTC

### 🚫 What Is Mocked or Unavailable on Testnet

- **Real BTC settlement:** No real Bitcoin is involved
- **Production Attestor SLAs:** Testnet Attestors do not carry the same uptime guarantees
- **Mainnet fee structure:** Fees on testnet may not reflect production costs

---

## Important Operational Notes

<aside>
⚠️

**Testnet may be reset without notice.** Do not rely on testnet state for production planning. Testnet CBTC balances and transaction history may not persist across resets.

</aside>

- Testnet is for **development and testing only**
- Do not use testnet data for compliance, reporting, or production decisions
- Testnet performance is not indicative of mainnet performance

---

## Migrate from CBTC Testnet to Mainnet: Step-by-Step Checklist

When your testnet integration is working, the migration to mainnet involves:

1. **Update your participant config** to point at the mainnet Canton network
2. **Update Instrument IDs** to mainnet values (see **API Reference**)
3. **Update Coordinator URL** to [`https://mainnet.dlc.link/attestor-1`](https://mainnet.dlc.link/attestor-1)
4. **Use real BTC** for minting — the flow is identical
5. **Review authentication** — ensure your production OIDC provider is configured
6. **Test with a small amount first** — mint a minimal amount of CBTC on mainnet before going live

---

## Devnet

There is also a **devnet** environment for earlier-stage experimentation. Devnet is less stable than testnet and may be updated more frequently.

**Coordinator URL:** [`https://devnet.dlc.link/attestor-2`](https://devnet.dlc.link/attestor-2)

```json
{
    "instrument_id": {
        "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
        "id": "CBTC"
    },
    "registry_url": "https://api.utilities.digitalasset-dev.com"
}
```

---

## Troubleshooting

| Issue | Resolution |
| --- | --- |
| Faucet not working | Use Bron Wallet testnet setup (Option 2) or contact BitSafe dev team |
| Testnet CBTC balance disappeared | Testnet may have been reset — request new tokens from the faucet |
| Minting on testnet takes too long | Check Bitcoin testnet block times — they can be irregular |
| Cannot connect to testnet participant | Verify your participant config points to the correct testnet endpoints |

**Support:** #cbtc-ecosystem (Slack) or [support@bitsafe.finance](mailto:support@bitsafe.finance)

---

<aside>
🔴

**⚙️ Engineering Review Required**

Testnet environment details (Attestor set size, reset policy, coordinator URLs) must be confirmed by Engineering (Jesse or Robert) before publication. Some details are drafted from internal docs and may not reflect the latest testnet configuration.

</aside>