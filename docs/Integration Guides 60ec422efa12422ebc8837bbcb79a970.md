# Integration Guides

Responsible: Max Webster-Dowsing
Created: February 10, 2026 4:31 PM
Created By: Max Webster-Dowsing
Last Edited: February 17, 2026 4:23 PM
Last Edited By: Aki Balogh
Pillars: CBTC (https://www.notion.so/CBTC-2c3636dd0ba580cf8739cd330148f78a?pvs=21)
Priority Level: Medium
Product: CBTC
Projects: CBTC Documentation Overhaul (https://www.notion.so/CBTC-Documentation-Overhaul-56ac9d22e884416f8f5db5bb7ead1d04?pvs=21)
Status: In Review
Status Update: Not started
Type: Guide

<aside>
👥

**Audience:** App Developers integrating CBTC into DeFi protocols, wallets, custody solutions, or trading systems.

</aside>

<aside>
⚠️

**API Disclaimer:** CBTC APIs are subject to change. Label all examples with the SDK version and DAR version they were tested against.

</aside>

---

## Overview

This page provides integration patterns for common CBTC use cases. Each pattern includes architecture notes, key considerations, and pointers to relevant code. For API details, see the **API Reference**. For authentication setup, see the **Authentication Guide**.

---

## Integration Pattern 1: DeFi Protocol

**Use case:** Build a DEX, lending platform, or liquidity pool using CBTC as collateral.

### Architecture

1. Your protocol runs on a Canton participant node with CBTC DAR files installed
2. Users deposit CBTC into your protocol's Canton party via a `Transfer` choice
3. Your protocol logic (Daml contracts) manages positions, collateral, and settlement
4. Users withdraw CBTC back to their own party when exiting

### Key Considerations

- **UTXO management:** Each transfer creates UTXOs. Keep below 10 per party. Use `cbtc-lib` consolidation functions.
- **Instrument ID:** Fetch dynamically — see **Instrument ID Management**
- **Privacy:** Canton transactions are private by default. Only parties to a contract see its details. This eliminates MEV.
- **Transfer costs:** ~$3–5 per CBTC transfer on Canton currently. Factor this into your protocol economics.

### Example Partners

- **Bron** — BTC-CBTC and CC-CBTC swapping on Canton
- **Elk Capital Markets / Triangle** — OTC and app-based CBTC trading
- **Silvana** — DEX/trading venue on Canton

---

## Integration Pattern 2: Wallet or Custody Solution

**Use case:** Support CBTC in an institutional-grade wallet or custody platform.

### Architecture

1. Wallet connects to a Canton participant via the Ledger API
2. Authentication via OIDC (Keycloak supported, Auth0 community example available)
3. CBTC balances queried via `state-queries` endpoint
4. Transfers executed via `Transfer` choice on CBTC token contracts

### Supported Wallets (Current Ecosystem)

- **Loop Wallet** — Canton-native wallet with CBTC support
- **Console / Zoro Wallet** — Canton wallet with API access
- **Bron Wallet** — Multi-party wallet with testnet support
- **WalletConnect** — For dApp-to-wallet connections

### Key Considerations

- **External signing:** Available for integration with custody providers (DFNS, Fordefi, Ledger)
- **Party creation at scale:** If creating 10+ parties, use the Ledger API directly rather than wallet UI — see [Canton docs](https://docs.digitalasset.com/build/3.4/tutorials/json-api/canton_and_the_json_ledger_api_ts.html#allocating-a-party)
- **CORS:** If your wallet makes browser-based API calls, configure CORS on your ingress

---

## Integration Pattern 3: Trading System

**Use case:** Build spot trading, perpetual contracts, options, or structured products with CBTC.

### Why Canton for Trading

- **No public mempool** — positions are not visible to other participants, eliminating front-running and sandwich attacks (MEV)
- **Private transactions** — only parties to a trade see the details
- **Audit-ready** — Canton's privacy model supports selective disclosure for compliance

### Architecture

1. Trading engine runs as Daml contracts on Canton
2. CBTC used as settlement or collateral asset
3. Counterparty discovery and matching handled by your protocol
4. Settlement is atomic — either both sides complete or neither does

### Example: Options on CBTC

CBTC holders can write covered CALL options, earning premium income while maintaining BTC exposure. Settlement uses Canton's atomic dual-token transfer — the buyer receives the underlying asset while the seller receives payment, atomically.

---

## Integration Pattern 4: Minting Integration

**Use case:** Offer CBTC minting as a service to your users.

### Three Options

| Option | Description | Effort |
| --- | --- | --- |
| **1. Direct API** | Install CBTC DAR, call Canton APIs to mint/redeem | Low — a few hours |
| **2. Self-hosted UI** | Install DAR + BitSafe minting UI locally | Medium — more maintenance |
| **3. Hosted UI** *(coming soon)* | Use BitSafe's centrally hosted UI against your validator | Minimal — config only |

For Option 1, see the **Minting and Burning Guide** and **API Reference**.

For Options 2 and 3, contact BitSafe for setup details.

---

## Getting Started

1. **Set up testnet first** — see **Testnet Guide**
2. **Install DAR files** — [GitHub](https://github.com/DLC-link/cbtc-lib/tree/main/cbtc-dars)
3. **Use cbtc-lib** — [GitHub](https://github.com/DLC-link/cbtc-lib) for type-safe Rust integration
4. **Review examples** — [GitHub examples](https://github.com/DLC-link/cbtc-lib/tree/main/examples)

**Need help?** Reach out via #cbtc-ecosystem (Slack) or [support@bitsafe.finance](mailto:support@bitsafe.finance)

---

<aside>
🔴

**⚙️ Engineering Review Required.** Integration patterns and partner references should be reviewed by Engineering (Jesse) and BD before publication. Wallet integration details may have changed.

</aside>