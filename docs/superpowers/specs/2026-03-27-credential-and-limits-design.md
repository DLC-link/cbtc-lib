# Credential and Min/Max Limits Support for cbtc-lib

**Date:** 2026-03-27
**Branch:** `feature/credential-and-min-max-support` (off `release/v1.2.0`)
**Related PRs:**
- Canton Daml credential checks: https://github.com/DLC-link/canton/pull/59
- Attestor-stack credential support: https://github.com/DLC-link/dlc-attestor-stack/pull/434
- Canton Daml transaction limits: https://github.com/DLC-link/canton/pull/61
- Attestor-stack limits governance: https://github.com/DLC-link/dlc-attestor-stack/pull/438

## Overview

CBTC release v1.2.0 introduces two new features at the Daml contract level:

1. **Minter credentials** — users must hold a `Credential` contract (issued by the registrar) with the claim `hasCBTCRole: Minter` to perform CBTC operations (account creation, withdrawals).
2. **Transaction limits** — deposit and withdraw accounts can have `Optional Limits` with `minAmount` and `maxAmount`, enforced on-ledger.

This spec covers the changes needed in cbtc-lib to support both features.

## Feature 1: Credential Support

### Background

The Canton Network Utility Credential system (`utility-credential-v0`, `utility-credential-app-v0`) provides on-ledger verifiable credentials. The CBTC Daml model now requires a valid Minter credential for:

- `CBTCDepositAccountRules_CreateDepositAccount` — `credentialCids: Vec<String>` (required)
- `CBTCWithdrawAccountRules_CreateWithdrawAccount` — `credentialCids: Vec<String>` (required)
- `CBTCWithdrawAccount_Withdraw` — `credentialCids: Option<Vec<String>>` (required, but Optional in Daml)
- `CBTCDepositAccount_CompleteDeposit` — `credentialCids: Option<Vec<String>>` (attestor-side only, not user-facing)

### Credential lifecycle (user's perspective)

1. Attestor network offers credential via governance (`OfferFreeCredential` on registrar's `UserService`)
2. User polls the ledger for pending `CredentialOffer` contracts
3. User accepts by exercising `UserService_AcceptFreeCredentialOffer` on their own `UserService` contract
4. User queries their active `Credential` contracts to get CIDs
5. User includes credential CIDs when submitting CBTC operations

The user cannot initiate credential issuance — the flow is issuer-initiated (offer → accept).

### Daml contract details

**`Credential`** (from `utility-credential-v0`):
- `issuer: Party` — the registrar/decparty
- `holder: Party` — the user
- `id: Text` — e.g., `"holder-credential"`
- `description: Text`
- `claims: [Claim]` — for CBTC: `[Claim { subject: holder_party, property: "hasCBTCRole", value: "Minter" }]`
- `validFrom: Optional Time`
- `validUntil: Optional Time`
- `observers: Set Party`

**`CredentialOffer`** (from `utility-credential-app-v0`):
- `operator: Party`
- `issuer: Party`
- `holder: Party`
- `dso: Party`
- `id: Text`
- `description: Text`
- `claims: [Claim]`
- `billingParams: Optional BillingParams` — `None` for free credentials

**`UserService`** (from `utility-credential-app-v0`):
- `operator: Party`
- `user: Party`
- `dso: Party`
- Key choice: `UserService_AcceptFreeCredentialOffer { credentialOfferCid }` — controlled by `user`

### Template IDs

```
CredentialOffer: #utility-credential-app-v0:Utility.Credential.App.V0.Model.Offer:CredentialOffer
Credential:      #utility-credential-v0:Utility.Credential.V0.Credential:Credential
UserService:     #utility-credential-app-v0:Utility.Credential.App.V0.Service.User:UserService
```

### Prerequisites

- The user must have a `UserService` contract (created during Canton Network utility onboarding). Without this, credential acceptance will fail. The `find_user_service` helper returns a clear error if no `UserService` is found.
- The attestor network must have offered a credential to the user via governance before the user can accept it.

### Visibility notes

- `CredentialOffer` has `signatory operator, issuer` and `observer holder`. Fetching by the holder's party works because the holder is an observer.
- `Credential` has `signatory issuer, holder`. Fetching by the holder's party works because the holder is a signatory.
- No disclosed contracts are needed for `UserService_AcceptFreeCredentialOffer` — the participant has visibility on the `CredentialOffer` because the holder is an observer.

### New module: `src/credentials.rs`

Template ID constants are defined within this module (not in `mint_redeem/constants.rs`) since they are only used here.

#### Models

All models derive `Debug, Clone, Serialize, Deserialize` to match existing cbtc-lib conventions.

```rust
/// A credential offer pending acceptance by the holder
pub struct CredentialOffer {
    pub contract_id: String,
    pub template_id: String,
    pub created_event_blob: String,
    pub issuer: String,
    pub holder: String,
    pub id: String,
    pub description: String,
    pub claims: Vec<Claim>,
}

/// A claim within a credential
pub struct Claim {
    pub subject: String,
    pub property: String,
    pub value: String,
}

/// An active credential held by the user
pub struct UserCredential {
    pub contract_id: String,
    pub template_id: String,
    pub issuer: String,
    pub holder: String,
    pub id: String,
    pub description: String,
    pub claims: Vec<Claim>,
}
```

#### Query functions

```rust
/// Fetch pending credential offers for the user (where user is the holder)
pub async fn list_credential_offers(params: ListCredentialOffersParams) -> Result<Vec<CredentialOffer>, String>

/// Fetch active credentials for the user (where user is the holder)
pub async fn list_credentials(params: ListCredentialsParams) -> Result<Vec<UserCredential>, String>
```

Both functions:
1. Get ledger end offset
2. Create template filter for the relevant template ID
3. Fetch active contracts for the user's party
4. Parse `createArgument` to extract fields
5. Filter: for offers, where `holder == party`; for credentials, where `holder == party`

**Note:** These return ALL credentials/offers visible to the party, not just CBTC-related ones. The user may hold credentials from other utilities. Examples should demonstrate filtering for CBTC Minter credentials (e.g., by checking `claims` contains `property: "hasCBTCRole", value: "Minter"`).

#### Accept function

```rust
/// Accept a free credential offer by exercising UserService_AcceptFreeCredentialOffer
pub async fn accept_credential_offer(params: AcceptCredentialOfferParams) -> Result<UserCredential, String>
```

Parameters:
- `ledger_host: String`
- `party: String` (the holder)
- `access_token: String`
- `user_service_contract_id: String` — the user's `UserService` contract ID
- `user_service_template_id: String`
- `credential_offer_cid: String` — the `CredentialOffer` contract ID to accept

Steps:
1. Build an `ExerciseCommand` on the `UserService` contract with choice `UserService_AcceptFreeCredentialOffer` and argument `{ credentialOfferCid }`
2. Submit via `submit::wait_for_transaction_tree`
3. Parse the response to extract the created `Credential` contract from `eventsById` — match by template ID suffix `:Utility.Credential.V0.Credential:Credential`
4. Return `UserCredential`

#### Finding the UserService contract

A helper function to find the user's `UserService` contract:

```rust
/// Find the UserService contract for a given user party
pub async fn find_user_service(params: FindUserServiceParams) -> Result<UserServiceInfo, String>
```

Returns `UserServiceInfo { contract_id, template_id, operator, user, dso }`.

The function queries `UserService` contracts by the party and filters where `user == party` (not `operator == party`), since the party may have visibility on other `UserService` contracts (e.g., the registrar's).

### Modified operations

#### `src/mint_redeem/mint.rs`

`CreateDepositAccountParams`:
```rust
pub credential_cids: Vec<String>,  // NEW — required
```

In the choice argument JSON, add:
```json
{
  "owner": "...",
  "credentialCids": ["cid1", "cid2"]
}
```

Note: `credentialCids` wraps into `Some [...]` on the Daml side (the Daml field is `credentialCids: Optional [ContractId Credential]`). Passing a JSON array maps to `Some [...]`.

#### `src/mint_redeem/redeem.rs`

`CreateWithdrawAccountParams`:
```rust
pub credential_cids: Vec<String>,  // NEW — required
```

Choice argument JSON:
```json
{
  "owner": "...",
  "destinationBtcAddress": "...",
  "credentialCids": ["cid1"]
}
```

`SubmitWithdrawParams`:
```rust
pub credential_cids: Option<Vec<String>>,  // NEW
```

Choice argument JSON (added to existing):
```json
{
  "tokens": [...],
  "amount": "...",
  "burnMintFactoryCid": "...",
  "extraArgs": {...},
  "credentialCids": ["cid1"]
}
```

### Breaking changes

Adding `credential_cids: Vec<String>` as a required field to `CreateDepositAccountParams` and `CreateWithdrawAccountParams` is a breaking change for existing callers. This is acceptable because:
- This is a v1.2.0 feature branch, not a patch on an existing release
- The Daml model requires credentials — callers must update to work with v1.2.0 contracts
- Without credentials, account creation will fail on-ledger

## Feature 2: Min/Max Transaction Limits

### Background

PR #61 adds `Optional Limits` to deposit and withdraw account contracts:

```
Limits {
  minAmount: Optional Decimal,
  maxAmount: Optional Decimal
}
```

- Rules contracts hold default limits inherited by new accounts at creation
- Account contracts hold active limits enforced at exercise time
- `checkLimits` is called in `CompleteDeposit` and `Withdraw` choices
- Limits are updated via governance-controlled `UpdateLimits` choices (not user-facing)
- `Limits with minAmount = None, maxAmount = None` normalizes to `None` (no limits)

### Model changes in `src/mint_redeem/models.rs`

```rust
/// Transaction limits for deposit/withdraw operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(rename = "minAmount")]
    pub min_amount: Option<String>,  // Decimal as string to preserve precision
    #[serde(rename = "maxAmount")]
    pub max_amount: Option<String>,
}
```

Add to `DepositAccount`:
```rust
pub limits: Option<Limits>,
```

Add to `WithdrawAccount`:
```rust
pub limits: Option<Limits>,
```

Add to `DepositAccountStatus`:
```rust
pub limits: Option<Limits>,
```

This ensures users who call `get_deposit_account_status` can see limits without a separate query.

#### Parsing from `createArgument`

The Daml field is `limits: Optional Limits`. In the JSON `createArgument`:
- Key absent or `"limits": null` → `None` (no limits enforced)
- `"limits": { "minAmount": "0.001", "maxAmount": null }` → `Some(Limits { min_amount: Some("0.001"), max_amount: None })`
- `"limits": { "minAmount": null, "maxAmount": "10.0" }` → `Some(Limits { min_amount: None, max_amount: Some("10.0") })`

Parse in `from_active_contract` using:
```rust
let limits = args.get("limits")
    .and_then(|v| if v.is_null() { None } else { serde_json::from_value::<Limits>(v.clone()).ok() });
```

### Client-side pre-check

Add to `src/mint_redeem/models.rs` or a utility location:

```rust
/// Check if an amount is within the account's limits.
/// Returns Ok(()) if within limits or no limits set,
/// Err with a descriptive message otherwise.
pub fn check_limits(operation: &str, amount: f64, limits: &Option<Limits>) -> Result<(), String> {
    if let Some(lim) = limits {
        if let Some(min) = &lim.min_amount {
            let min_val: f64 = min.parse().map_err(|e| format!("Invalid min_amount: {}", e))?;
            if amount < min_val {
                return Err(format!(
                    "{} amount {} is below minimum {}",
                    operation, amount, min
                ));
            }
        }
        if let Some(max) = &lim.max_amount {
            let max_val: f64 = max.parse().map_err(|e| format!("Invalid max_amount: {}", e))?;
            if amount > max_val {
                return Err(format!(
                    "{} amount {} exceeds maximum {}",
                    operation, amount, max
                ));
            }
        }
    }
    Ok(())
}
```

This mirrors the Daml `checkLimits` function and gives users a clear error before submitting a transaction that would be rejected on-ledger.

## Examples

### New: `examples/credentials.rs`

Demonstrates the full credential lifecycle:

1. Authenticate with Keycloak
2. Check for existing credentials (`list_credentials`)
3. If none, list pending offers (`list_credential_offers`)
4. Accept an offer (`accept_credential_offer`)
5. Verify the credential was created (`list_credentials`)
6. Use the credential CID in a deposit account creation

### Updated: `examples/mint_cbtc_flow.rs`

Add credential fetching before `create_deposit_account`:

1. Fetch credentials via `list_credentials`
2. Extract credential CIDs
3. Pass to `CreateDepositAccountParams { credential_cids, ... }`

### Updated: `examples/redeem_cbtc_flow.rs`

1. Fetch credentials via `list_credentials`
2. Pass credential CIDs to `CreateWithdrawAccountParams`
3. Before `submit_withdraw`, call `check_limits("Withdraw", amount, &withdraw_account.limits)` to pre-validate
4. Pass credential CIDs to `SubmitWithdrawParams`

## Cargo.toml changes

No new dependencies expected. The existing `ledger`, `common`, `serde_json`, `reqwest` dependencies cover the needed functionality.

A new `[[example]]` entry:
```toml
[[example]]
name = "credentials"
path = "examples/credentials.rs"
```

## Files changed summary

| File | Change |
|------|--------|
| `src/lib.rs` | Add `pub mod credentials;` |
| `src/credentials.rs` | **New** — credential query + accept module (includes template ID constants) |
| `src/mint_redeem/models.rs` | Add `Limits` (with serde rename), `limits` field to `DepositAccount` and `WithdrawAccount`, add `check_limits` |
| `src/mint_redeem/mint.rs` | Add `credential_cids` to `CreateDepositAccountParams`, include in choice argument |
| `src/mint_redeem/redeem.rs` | Add `credential_cids` to `CreateWithdrawAccountParams` and `SubmitWithdrawParams`, include in choice arguments |
| `examples/credentials.rs` | **New** — credential lifecycle example (with CBTC Minter claim filtering) |
| `examples/mint_cbtc_flow.rs` | Fetch + pass credentials |
| `examples/redeem_cbtc_flow.rs` | Fetch + pass credentials, add limit pre-check |
| `Cargo.toml` | Add `credentials` example entry |

## Test impact

Existing integration tests in `mint.rs` and `redeem.rs` will fail to compile after adding `credential_cids` to the Params structs. These tests must be updated to pass credential CIDs. Since the tests require live Canton credentials and a running network, they will also need a valid Minter credential on the test account. The test updates are mechanical (add the field) but the test environment must have credentials issued to the test party.
