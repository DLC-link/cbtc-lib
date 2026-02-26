<aside>
⚠️

**API Disclaimer.** CBTC APIs have no formal versioning policy today. All endpoints and library interfaces described in this guide are **subject to change**. Breaking changes are communicated via #cbtc-ecosystem and the changelog. This disclaimer will be updated once a formal versioning and stability policy is established.

</aside>

This step-by-step guide walks you through minting your first CBTC (wrapped Bitcoin) on the Canton Network. You will authenticate with Keycloak, create a deposit account, send BTC to a Taproot address, and receive 1:1 backed CBTC on your Canton participant node. The full process takes about 15 minutes of active work plus ~60 minutes of Bitcoin confirmation time.

<aside>
🎯

**What you will accomplish**

- Authenticate to the Canton Network via Keycloak
- Create a CBTC deposit account
- Obtain a Bitcoin deposit address
- Send BTC and wait for confirmation
- Verify your CBTC balance
- Send CBTC to another party (two-phase transfer)
</aside>

---

## Prerequisites for Minting CBTC

Before you begin minting wrapped Bitcoin on Canton, make sure you have the following:

| Requirement | Description |
| --- | --- |
| **Canton participant node** | A running Canton participant node connected to the network. See [Canton documentation](https://docs.digitalasset.com/canton) for setup. |
| **DA Registry Utility** | Installed and configured. See [Digital Asset Utilities docs](https://docs.digitalasset.com/utilities/mainnet/index.html). |
| **Keycloak credentials** | A valid Keycloak username, password, client ID, and client secret for your environment. |
| **Party ID** | Your Canton Party ID, obtained during onboarding. |
| **Rust toolchain** | If using cbtc-lib (Rust). Install via [rustup.rs](http://rustup.rs). |
| **BTC to deposit** | Real BTC (mainnet) or testnet BTC (testnet). There is no faucet for CBTC. You mint CBTC by depositing BTC through the same flow on both networks. |

---

## Choose Your CBTC Environment: Testnet or Mainnet

CBTC is available on three environments. **Start with testnet** for experimentation, then move to mainnet for production.

<aside>
🧪

**Testnet vs. Mainnet: what is identical and what differs**

- **Identical:** DAR file, API surface, mint/burn flows, governance model, two-phase transfer mechanics
- **Differs:** Attestor set (smaller on testnet), confirmation times (may be faster), Instrument IDs (different from mainnet), faucet-only BTC on testnet (no real value)
- **Mocked or unavailable on testnet:** Real BTC settlement, production Attestor SLAs, mainnet fee structure
- **Operational note:** Testnet may be reset without notice. Testnet CBTC balances and transaction history may not persist across resets. Do not rely on testnet state for production planning. *Exact reset schedule and data persistence details to be confirmed with Engineering.*
</aside>

### Environment Configuration

| Variable | Devnet | Testnet | Mainnet |
| --- | --- | --- | --- |
| `REGISTRY_URL` | [`https://api.utilities.digitalasset-dev.com`](https://api.utilities.digitalasset-dev.com) | [`https://api.utilities.digitalasset-staging.com`](https://api.utilities.digitalasset-staging.com) | [`https://api.utilities.digitalasset.com`](https://api.utilities.digitalasset.com) |
| `ATTESTOR_URL` | [`https://attestor.bitsafe.dev`](https://attestor.bitsafe.dev) | [`https://attestor.bitsafe.testnet`](https://attestor.bitsafe.testnet) | [`https://attestor.bitsafe.com`](https://attestor.bitsafe.com) |
| `DECENTRALIZED_PARTY_ID` | *Environment-specific. Provided during onboarding.* | *Environment-specific. Provided during onboarding.* | *Environment-specific. Provided during onboarding.* |

Set these as environment variables before running any commands:

```bash
export REGISTRY_URL="https://api.utilities.digitalasset-staging.com"
export ATTESTOR_URL="https://attestor.bitsafe.testnet"
export DECENTRALIZED_PARTY_ID="your-party-id-here"
export KEYCLOAK_URL="your-keycloak-url"
export KEYCLOAK_CLIENT_ID="your-client-id"
export KEYCLOAK_CLIENT_SECRET="your-client-secret"
export KEYCLOAK_USERNAME="your-username"
export KEYCLOAK_PASSWORD="your-password"
export LEDGER_HOST="your-ledger-host"
export LEDGER_PORT="your-ledger-port"
```

---

## Step 1: Authenticate with Keycloak

All CBTC operations require a valid Keycloak access token. The `canton-lib` crate provides a helper for this.

### Using cbtc-lib (Rust)

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

### Using the Canton API directly

```bash
curl -X POST "${KEYCLOAK_URL}/realms/canton/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password" \
  -d "client_id=${KEYCLOAK_CLIENT_ID}" \
  -d "client_secret=${KEYCLOAK_CLIENT_SECRET}" \
  -d "username=${KEYCLOAK_USERNAME}" \
  -d "password=${KEYCLOAK_PASSWORD}"
```

Save the `access_token` from the response. You will pass it as a Bearer token in all subsequent API calls.

---

## Step 2: Create a Deposit Account

A deposit account maps your Canton Party ID to a unique Bitcoin deposit address. You only need to create this once; the address can be reused for future deposits.

### Using cbtc-lib

```rust
use mint_redeem::mint;

let deposit_account = mint::create_deposit_account(
    &ledger_client,
    &party_id,
    &token,
).await?;
```

### Using the Canton API directly

Submit a `CreateDepositAccount` command to the Canton Ledger API v2 endpoint:

```bash
curl -X POST "https://${LEDGER_HOST}:${LEDGER_PORT}/v2/commands/submit" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "templateId": "#cbtc-deposit-account-template-id",
      "payload": {
        "party": "'${PARTY_ID}'"
      }
    }]
  }'
```

---

## Step 3: Get Your Bitcoin Deposit Address

Once the deposit account is created, retrieve the Bitcoin address associated with it. This address is deterministically derived from your Canton Party ID using Taproot script construction.

### Using cbtc-lib

```rust
let btc_address = mint::get_deposit_address(
    &ledger_client,
    &party_id,
    &token,
).await?;

println!("Send BTC to: {}", btc_address);
```

**Important:** This address can be reused indefinitely for future deposits. You can also request additional deposit addresses if needed.

---

## Step 4: Send BTC

Send Bitcoin to the deposit address using any standard Bitcoin wallet or tooling. There is no minimum deposit enforced at the protocol level, but check with BitSafe for any operational minimums.

```
Bitcoin address: [your deposit address from Step 3]
```

After sending, you need to wait for **6 Bitcoin block confirmations** before the attestor network will process the deposit.

---

## Step 5: Wait for Confirmations and Auto-Minting

Once your BTC transaction reaches 6 confirmations:

1. The **attestor network** detects the confirmed deposit
2. Each attestor independently verifies the transaction
3. When a threshold of attestors confirm (via `ConfirmDepositAction` on Canton's governance module), CBTC is **automatically minted** to your Canton Party
4. No further action is required from you

This process typically takes 60 to 90 minutes, depending on Bitcoin block times.

---

## Step 6: Check Your CBTC Balance

### Using cbtc-lib

```rust
use cbtc::active_contracts;

let holdings = active_contracts::get_active_contracts(
    &ledger_client,
    &party_id,
    &token,
).await?;

for holding in holdings {
    println!("Amount: {}", holding.amount);
}
```

### Using the Canton API directly

Query active contracts filtered by the CBTC token holding interface:

```bash
curl -X POST "https://${LEDGER_HOST}:${LEDGER_PORT}/v2/state/active-contracts" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "filter": {
      "interfaceFilters": [{
        "interfaceId": "#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding"
      }]
    },
    "activeAtOffset": ""
  }'
```

<aside>
💡

**UTXO model.** CBTC uses a UTXO model similar to Bitcoin. Your balance may be spread across multiple holding contracts (soft limit of 10 UTXOs per party per token type). Use the `cbtc::consolidate` module to merge UTXOs, or `cbtc::split` to divide them.

</aside>

---

## Step 7: Transfer CBTC Between Canton Parties

CBTC transfers use a **two-phase model**: the sender creates an offer, and the receiver accepts it. This ensures both parties explicitly consent to the transfer.

### Phase 1: Create a transfer offer (sender)

```rust
use cbtc::transfer;

let transfer_offer = transfer::send(
    &ledger_client,
    &sender_party_id,
    &receiver_party_id,
    amount,
    &token,
).await?;
```

### Phase 2: Accept the transfer (receiver)

```rust
use cbtc::accept;

let accepted = accept::accept_transfer(
    &ledger_client,
    &receiver_party_id,
    &transfer_contract_id,
    &token,
).await?;
```

### Using the Canton API directly

**Submit a transfer:**

```bash
curl -X POST "https://${LEDGER_HOST}:${LEDGER_PORT}/v2/commands/submit" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "templateId": "#splice-api-token-transfer-instruction-v1:Splice.Api.Token.TransferInstructionV1:TransferInstruction",
      "payload": {
        "sender": "'${SENDER_PARTY_ID}'",
        "receiver": "'${RECEIVER_PARTY_ID}'",
        "amount": "'${AMOUNT}'"
      }
    }]
  }'
```

**Accept a transfer:**

```bash
curl -X POST "https://${LEDGER_HOST}:${LEDGER_PORT}/v2/commands/submit" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "exerciseCommand": {
        "templateId": "#splice-api-token-transfer-instruction-v1:Splice.Api.Token.TransferInstructionV1:TransferInstruction",
        "contractId": "'${TRANSFER_CONTRACT_ID}'",
        "choice": "Accept"
      }
    }]
  }'
```

---

## Redeem CBTC: Convert Wrapped Bitcoin Back to BTC

To convert CBTC back to BTC:

1. **Burn CBTC** on Canton using the `mint_redeem::redeem` module
2. **Create a withdraw request** specifying your Bitcoin destination address
3. The **attestor network** detects the burn and constructs a Bitcoin transaction
4. Attestors sign the transaction via threshold signing (FROST)
5. The BTC transaction is broadcast to the Bitcoin network

```rust
use mint_redeem::redeem;

let withdraw = redeem::burn_and_withdraw(
    &ledger_client,
    &party_id,
    &btc_destination_address,
    amount,
    &token,
).await?;
```

---

## Additional Operations

The `cbtc-lib` library provides several utility modules for managing your CBTC holdings:

| Module | Purpose |
| --- | --- |
| `cbtc::batch` | Batch operations for sending CBTC to multiple recipients |
| `cbtc::distribute` | Distribute CBTC across multiple parties |
| `cbtc::consolidate` | Merge multiple UTXO holdings into fewer contracts |
| `cbtc::split` | Split a single holding into multiple UTXOs |
| `cbtc::active_contracts` | Query your current CBTC holdings |

---

## Troubleshooting

- My deposit has not been minted after 90 minutes
    - Verify the BTC transaction has at least 6 confirmations on a block explorer
    - Confirm you sent to the correct deposit address (from Step 3)
    - Check that your Canton participant node is connected and syncing
    - Escalation: contact [support@bitsafe.finance](mailto:support@bitsafe.finance) or post in #cbtc-ecosystem on Slack
- Transfer offer is not appearing for the receiver
    - The receiver must be registered in the DA Registry with a valid credential
    - Confirm the receiver's Party ID is correct
    - Check that both parties are connected to the same Canton sync domain
- "Insufficient holdings" error when sending
    - CBTC uses a UTXO model. You may need to consolidate holdings first using `cbtc::consolidate`
    - Check your balance with `cbtc::active_contracts` to verify available amounts
- Authentication token expired
    - Keycloak tokens have a limited lifetime. Re-authenticate using Step 1 before retrying the operation

---

## Next Steps

- **API Reference:** Full documentation of Canton Ledger API endpoints for CBTC operations *(coming soon)*
- **SDK Reference:** Complete `cbtc-lib` and `canton-lib` module documentation *(coming soon)*
- **Authentication Guide:** Detailed Keycloak setup and Auth0 community example *(coming soon)*
- **Integration Examples:** Real-world code showing CBTC in DeFi protocols, wallets, and trading systems *(coming soon)*

---

<aside>
📧

**Need help?** Reach out to [support@bitsafe.finance](mailto:support@bitsafe.finance) or post in **#cbtc-ecosystem** on Slack. For urgent technical issues, tag the engineering team directly in the Slack channel.

</aside>

<aside>
⚠️

**Engineering review required before publication.** The code examples in this guide are based on `cbtc-lib` and `canton-lib` README module and function names, but some may need verification against the actual source once Jesse or Ferenc can review. The curl examples for the Canton Ledger API are structural patterns derived from the README documentation, not copy-paste production-ready calls. All code samples, endpoint paths, and payload structures must be validated against live deployments before this guide is published externally.

</aside>