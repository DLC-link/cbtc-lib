# Security Deep Dive

Responsible: Max Webster-Dowsing
Created: February 10, 2026 4:31 PM
Created By: Max Webster-Dowsing
Last Edited: February 11, 2026 10:12 PM
Last Edited By: Max Webster-Dowsing
Pillars: CBTC (https://www.notion.so/CBTC-2c3636dd0ba580cf8739cd330148f78a?pvs=21)
Priority Level: Medium
Product: CBTC
Projects: CBTC Documentation Overhaul (https://www.notion.so/CBTC-Documentation-Overhaul-56ac9d22e884416f8f5db5bb7ead1d04?pvs=21)
Status: In Review
Status Update: Not started
Type: Technical Spec

<aside>
👥

**Audience:** Internal Partner Engineers, security-focused developers, and technical evaluators conducting due diligence on CBTC's security model.

</aside>

---

## Overview

CBTC's security model is built on three pillars: **FROST threshold signatures** on the Bitcoin side, a **decentralised Attestor Network** bridging both chains, and **Daml smart contracts** governing all operations on Canton. This page covers each in detail.

---

## FROST Threshold Signatures

CBTC uses **FROST** (Flexible Round-Optimised Schnorr Threshold Signatures), a cryptographic protocol formalised in [Komlo & Goldberg, 2020](https://eprint.iacr.org/2020/852) and enabled on Bitcoin via the **Taproot** upgrade.

### How FROST Works

FROST is a **two-round signing protocol:**

1. **Round One (Commitment):** The Coordinator selects the message to be signed and the set of participating Attestors. Each Attestor generates fresh nonces and public commitments, sent to the Coordinator.
2. **Round Two (Signature Share):** The Coordinator broadcasts all commitments. Each Attestor verifies them, computes an individual signature share, and sends it back. The Coordinator aggregates shares into a single valid Schnorr signature.

### Why FROST for CBTC

- **Taproot-native:** FROST signatures are standard Schnorr signatures, compatible with any Taproot (P2TR) wallet. No special wallet support needed.
- **Indistinguishable on-chain:** A FROST threshold signature looks identical to a single-signer Schnorr signature. No one can determine from the blockchain that a threshold scheme was used.
- **Smaller and cheaper:** One aggregated signature regardless of threshold size, versus N signatures for traditional on-chain multisig. Lower transaction fees.
- **No single point of failure:** The signing key is never reconstructed. Each Attestor holds only a share.

### Security Properties

- **Unforgeability:** No coalition below the threshold can produce a valid signature, even with adaptive corruption (formally proven in the ePrint paper)
- **Robustness against forgery attacks:** FROST mitigates certain Schnorr-specific threshold forgery vectors
- **Forget-and-Forgive protection:** The resharing protocol includes acknowledgement steps preventing split-group attacks during key rotation

**Full paper:** [FROST: Flexible Round-Optimized Schnorr Threshold Signatures (ePrint 2020/852)](https://eprint.iacr.org/2020/852)

---

## Attestor Network

The Attestor Network is the decentralised security backbone of CBTC.

### Composition

- **9 pre-screened external node operators** (including established infrastructure providers like P2P and Everstake)
- **1 BitSafe-operated node**
- Each operator maintains **over $1B in assets under management**
- Every Attestor runs nodes on **both** the Bitcoin and Canton networks

### Responsibilities

Attestor responsibilities are **almost entirely automated:**

- Independent verification of Bitcoin transactions reaching 6 confirmations
- Submission of `ConfirmDepositAction` (for mints) and `ArchiveWithdrawRequest` (for burns) to the Canton governance module
- Participation in FROST threshold signing for Bitcoin withdrawal transactions
- Monitoring deposit accounts and withdrawal requests

The only manual process is **governance** — adding or removing Attestor nodes, which requires coordination between operators.

### Threshold Governance

- For critical actions (minting, burning), each Attestor submits confirmation **independently**
- Confirmations are recorded as Canton contracts
- Once the number of valid confirmations meets the **predefined threshold**, the Coordinator executes the action
- **No single party** — including BitSafe or the Coordinator — can unilaterally mint, burn, or move BTC

### Coordinator Role

The Coordinator is a service (which can be an Attestor or a separate non-signing entity) that:

- Executes periodic checks every **60–120 seconds**
- Monitors deposit accounts for new Bitcoin transactions
- Constructs Bitcoin transactions for withdrawals
- Submits governance actions to Canton
- Coordinates the FROST signing rounds

**The Coordinator cannot act unilaterally.** It facilitates the process but requires threshold approval for every action.

---

## Dual-Network Security Model

CBTC's security spans two networks simultaneously:

| Layer | Network | Security Mechanism |
| --- | --- | --- |
| **Bitcoin custody** | Bitcoin L1 | FROST threshold signatures (Taproot) |
| **Governance and coordination** | Canton | Daml contracts with threshold confirmation |
| **Token operations** | Canton | CIP-56 compliant Daml contracts |

The same Attestor network secures both layers, creating seamless security across both blockchains.

---

## Reliability and Safeguards

### Automatic Retry

If a Bitcoin transaction fails to broadcast, the Coordinator detects the failure during subsequent checks and rebroadcasts using stored transaction data.

### Idempotent Operations

Each withdrawal generates a unique transaction ID preventing double-spending, even with network-induced retries.

### Distributed Verification

No single Attestor can block or manipulate operations. The threshold system ensures continued operation even with some nodes offline.

---

## Trust and Threat Model

The CBTC system assumes an **honest majority** of the Attestor network. Key trust assumptions:

- A threshold of Attestors must be honest and online for the system to operate
- The Coordinator facilitates but cannot act unilaterally
- BitSafe operates one Attestor node but has no special privileges
- Canton's privacy model ensures transaction details are visible only to involved parties

### What Cannot Happen

- No single party (including BitSafe) can mint CBTC without genuine BTC deposits
- No single party can withdraw BTC without threshold approval
- No front-running or MEV — Canton has no public mempool

---

## Audit Reports

CBTC smart contracts have been audited by **Quantstamp:**

[**View Full Audit Report →**](https://certificate.quantstamp.com/full/cbtc/5d0d805e-8cf0-4a39-bf1a-0e94899b3c1c/index.html)

---

## Further Reading

- [FROST Whitepaper (ePrint 2020/852)](https://eprint.iacr.org/2020/852)
- [Canton Network Whitepaper](https://www.canton.network/whitepapers)
- [Canton Token Standard Docs](https://docs.dev.sync.global/app_dev/token_standard/index.html#api-references)

---

<aside>
🔴

**⚙️ Engineering Review Required.** Attestor network composition (current threshold, number of operators) and Coordinator behaviour must be validated by Engineering (Jesse or Robert) before publication.

</aside>