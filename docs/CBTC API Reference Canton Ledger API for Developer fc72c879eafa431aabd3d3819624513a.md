# CBTC API Reference: Canton Ledger API for Developers

Responsible: Max Webster-Dowsing
Created: February 10, 2026 4:31 PM
Created By: Max Webster-Dowsing
Last Edited: February 10, 2026 7:04 PM
Last Edited By: Max Webster-Dowsing
Pillars: CBTC (https://www.notion.so/CBTC-2c3636dd0ba580cf8739cd330148f78a?pvs=21)
Priority Level: High
Product: CBTC
Projects: CBTC Documentation Overhaul (https://www.notion.so/CBTC-Documentation-Overhaul-56ac9d22e884416f8f5db5bb7ead1d04?pvs=21)
Status: In Review
Status Update: Not started
Type: Technical Spec

<aside>
👥

**Audience:** App Developers integrating CBTC programmatically via the Canton Ledger API.

</aside>

<aside>
⚠️

**API Stability Notice:** All endpoints documented here are subject to change without notice. There is no formal versioning policy today. Breaking changes are communicated via #cbtc-ecosystem and the site changelog. Per-endpoint stability tags (stable, beta, experimental) will be introduced once a formal versioning policy is established.

</aside>

---

## Overview: Canton Ledger API for CBTC Operations

All CBTC operations (minting, burning, transferring wrapped Bitcoin) are performed through the **Canton Ledger API** (also called the JSON Ledger API). There is no separate "CBTC API." You interact with CBTC by exercising choices on Daml smart contracts running on your Canton participant node.

**Base URL:** `https://<your-participant-host>/v2/`

**Authentication:** Bearer token (JWT) from your OIDC provider. See the **Authentication Guide**.

**Content-Type:** `application/json` for all requests.

---

## Prerequisites

Before calling any CBTC API:

1. **Canton participant node** running and connected to the network
2. **CBTC DAR files** installed on your participant — [download from GitHub](https://github.com/DLC-link/cbtc-lib/tree/main/cbtc-dars)
3. **Valid JWT** from your OIDC provider (Keycloak officially supported)
4. **Party ID** allocated on your participant

---

## Endpoints

### Authentication

| Method | Endpoint | Description |
| --- | --- | --- |
| POST | `/v2/users` | Create a user on the Ledger API |
| GET | `/v2/users/{userId}` | Get user details |

### Deposit (Mint) Operations

| Method | Endpoint | Description |
| --- | --- | --- |
| POST | `/v2/commands` | Create a Deposit Account |
| POST | `/v2/commands` | Request a Bitcoin deposit address |
| POST | `/v2/event-queries` | Query pending deposits and confirmation status |
| POST | `/v2/state-queries` | Get CBTC balance for a party |

### Withdrawal (Burn) Operations

| Method | Endpoint | Description |
| --- | --- | --- |
| POST | `/v2/commands` | Initiate a CBTC burn / BTC withdrawal |
| POST | `/v2/event-queries` | Query withdrawal status |

### Transfer Operations

| Method | Endpoint | Description |
| --- | --- | --- |
| POST | `/v2/commands` | Transfer CBTC between parties |
| POST | `/v2/state-queries` | List active CBTC contracts for a party |

---

## CBTC Daml Smart Contract Templates

All CBTC operations use these Daml contract templates. You reference them by their fully qualified name in API calls.

| Template | Purpose | Key Choices |
| --- | --- | --- |
| `CBTC.Issuance:DepositAccount` | Represents a deposit account for minting | `RequestDepositAddress` |
| `CBTC.Issuance:DepositAddress` | A generated Bitcoin deposit address | (read-only, auto-created) |
| `CBTC.Issuance:PendingDeposit` | Tracks a deposit awaiting confirmations | (read-only, auto-managed) |
| `CBTC.Issuance:WithdrawRequest` | A request to burn CBTC and receive BTC | `InitiateWithdrawal` |
| `CBTC.Token:CBTC` | The CBTC token contract itself | `Transfer`, `Split`, `Merge` |

<aside>
💡

**Tip:** The `cbtc-lib` Rust library wraps all of these API calls with type-safe functions. We strongly recommend using it instead of raw API calls where possible. [View on GitHub →](https://github.com/DLC-link/cbtc-lib)

</aside>

---

## Example: Create a Deposit Account

```bash
curl -X POST "https://<your-participant>/v2/commands" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "createCommand": {
        "templateId": {
          "moduleName": "CBTC.Issuance",
          "entityName": "DepositAccount"
        },
        "createArguments": {
          "owner": "'$PARTY_ID'"
        }
      }
    }],
    "actAs": ["'$PARTY_ID'"],
    "commandId": "'$(uuidgen)'"
  }'
```

---

## Example: Request a Deposit Address

```bash
curl -X POST "https://<your-participant>/v2/commands" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "exerciseCommand": {
        "templateId": {
          "moduleName": "CBTC.Issuance",
          "entityName": "DepositAccount"
        },
        "contractId": "'$DEPOSIT_ACCOUNT_CONTRACT_ID'",
        "choice": "RequestDepositAddress",
        "choiceArgument": {}
      }
    }],
    "actAs": ["'$PARTY_ID'"],
    "commandId": "'$(uuidgen)'"
  }'
```

**Response** includes a Bitcoin Taproot (P2TR) address. Send BTC to this address to initiate minting.

---

## Example: Check CBTC Balance

```bash
curl -X POST "https://<your-participant>/v2/state-queries" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "activeContractSetQuery": {
      "templateIds": [{
        "moduleName": "CBTC.Token",
        "entityName": "CBTC"
      }],
      "activeAtOffset": "latest"
    },
    "actAs": ["'$PARTY_ID'"]
  }'
```

---

## CBTC Instrument ID Management: Devnet, Testnet, and Mainnet

CBTC uses the Canton Token Standard. To interact with CBTC programmatically, you need the correct **Instrument ID** for your target network.

<aside>
⚠️

**Instrument IDs differ across networks** (devnet, testnet, mainnet). Always fetch the latest from the metadata URL rather than hardcoding.

</aside>

### Devnet

```json
{
    "instrument_id": {
        "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
        "id": "CBTC"
    },
    "registry_url": "https://api.utilities.digitalasset-dev.com"
}
```

**Metadata:** [View](https://api.utilities.digitalasset-dev.com/api/token-standard/v0/registrars/cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff/registry/metadata/v1/instruments)

### Testnet

```json
{
    "instrument_id": {
        "admin": "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f",
        "id": "CBTC"
    },
    "registry_url": "https://api.utilities.digitalasset-staging.com"
}
```

**Metadata:** [View](https://api.utilities.digitalasset-staging.com/api/token-standard/v0/registrars/cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f/registry/metadata/v1/instruments)

### Mainnet

```json
{
    "instrument_id": {
        "admin": "cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262",
        "id": "CBTC"
    },
    "registry_url": "https://api.utilities.digitalasset.com"
}
```

**Metadata:** [View](https://api.utilities.digitalasset.com/api/token-standard/v0/registrars/cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262/registry/metadata/v1/instruments)

**Token Standard API Reference:** [Canton Token Standard Docs](https://docs.dev.sync.global/app_dev/token_standard/index.html#api-references)

<aside>
💡

**Polling pattern:** Instrument IDs can change due to network dynamics (e.g., DAR upgrades). Query the metadata URL periodically rather than hardcoding values. There is currently no push notification for ID changes — this is a known gap.

</aside>

---

## Rate Limits

There are no BitSafe-imposed rate limits on the Canton Ledger API. However:

- **Canton network throughput:** Transfers take a few seconds each. Approximately 500 transfers per 10-minute period is near the current practical limit.
- **Your participant node:** Performance depends on your infrastructure. Monitor node resource usage under load.

---

## API Error Handling for CBTC Operations

| Error | Cause | Resolution |
| --- | --- | --- |
| `401 Unauthorized` | Invalid or expired JWT | Re-authenticate with your OIDC provider |
| `404 Not Found` | Contract ID no longer active | Re-query for current contract IDs |
| `409 Conflict` | Duplicate command ID | Use a unique `commandId` per request |
| UTXO limit exceeded | Too many UTXOs for a party (max 10) | Consolidate UTXOs using cbtc-lib |

---

## CBTC SDK Reference: cbtc-lib (Rust)

For most integrations, we recommend using **cbtc-lib** (Rust) rather than raw API calls:

- **Repository:** [github.com/DLC-link/cbtc-lib](http://github.com/DLC-link/cbtc-lib)
- **Current version:** v0.0.1 (tagged after Dec 2025 restructure by Ferenc)
- **Lower-level library:** [github.com/DLC-link/canton-lib](http://github.com/DLC-link/canton-lib)
- **Code examples:** [github.com/DLC-link/cbtc-lib/tree/main/examples](http://github.com/DLC-link/cbtc-lib/tree/main/examples)

<aside>
💡

**Note:** cbtc-lib recently underwent a major restructure (December 2025). If you were using an earlier version, you will need to update your imports. See the [cleanup PR](https://github.com/DLC-link/cbtc-lib/pull/11) for migration details.

</aside>

---

<aside>
🔴

**⚙️ Engineering Review Required**

All endpoint details, Daml template names, and code examples in this document must be validated by Engineering (Jesse or Ferenc) against the actual implementation before publication. Template fully qualified names and API response shapes may differ.

</aside>