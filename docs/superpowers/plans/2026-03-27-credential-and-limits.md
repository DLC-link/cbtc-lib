# Credential and Min/Max Limits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Minter credential support (query, accept, include in operations) and transaction limit support (read from accounts, client-side pre-check) to cbtc-lib for CBTC v1.2.0.

**Architecture:** New `src/credentials.rs` module for credential operations (query offers, accept, query credentials, find UserService). Existing `mint_redeem` modules modified to accept credential CIDs in Params structs and include them in choice arguments. `Limits` struct and `check_limits` helper added to models. No new crate dependencies.

**Tech Stack:** Rust, Canton Ledger API (via `ledger` crate), `serde_json` for JSON construction/parsing, `common` crate for submission types.

**Spec:** `docs/superpowers/specs/2026-03-27-credential-and-limits-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/credentials.rs` | **New.** Credential query + accept module. Template ID constants, models (`Claim`, `CredentialOffer`, `UserCredential`, `UserServiceInfo`), param structs, functions (`list_credential_offers`, `list_credentials`, `accept_credential_offer`, `find_user_service`). |
| `src/lib.rs` | Add `pub mod credentials;` |
| `src/mint_redeem/models.rs` | Add `Limits` struct (with serde rename), `check_limits` helper, `limits` field to `DepositAccount`, `WithdrawAccount`, `DepositAccountStatus`. |
| `src/mint_redeem/mint.rs` | Add `credential_cids` to `CreateDepositAccountParams`, include in choice argument JSON. Update `get_deposit_account_status` to propagate limits. |
| `src/mint_redeem/redeem.rs` | Add `credential_cids` to `CreateWithdrawAccountParams` and `SubmitWithdrawParams`, include in choice argument JSON. |
| `examples/credentials.rs` | **New.** Example: credential lifecycle (list, accept, use). |
| `examples/mint_cbtc_flow.rs` | Update: fetch credentials before account creation. |
| `examples/redeem_cbtc_flow.rs` | Update: fetch credentials, check limits before withdraw. |
| `Cargo.toml` | Add `credentials` example entry. |

---

### Task 1: Add `Limits` struct and `check_limits` to models

**Files:**
- Modify: `src/mint_redeem/models.rs`

- [ ] **Step 1: Add `Limits` struct after existing imports**

At the top of `src/mint_redeem/models.rs`, after the existing `use` statements, add:

```rust
/// Transaction limits for deposit/withdraw operations.
/// Amounts are stored as strings to preserve decimal precision (Canton uses Numeric 10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(rename = "minAmount")]
    pub min_amount: Option<String>,
    #[serde(rename = "maxAmount")]
    pub max_amount: Option<String>,
}

/// Check if an amount is within the account's limits.
/// Returns Ok(()) if within limits or no limits set,
/// Err with a descriptive message otherwise.
pub fn check_limits(operation: &str, amount: f64, limits: &Option<Limits>) -> Result<(), String> {
    if let Some(lim) = limits {
        if let Some(min) = &lim.min_amount {
            let min_val: f64 = min
                .parse()
                .map_err(|e| format!("Invalid min_amount: {}", e))?;
            if amount < min_val {
                return Err(format!(
                    "{} amount {} is below minimum {}",
                    operation, amount, min
                ));
            }
        }
        if let Some(max) = &lim.max_amount {
            let max_val: f64 = max
                .parse()
                .map_err(|e| format!("Invalid max_amount: {}", e))?;
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

- [ ] **Step 2: Add `limits` to `DepositAccount` struct**

In `DepositAccount`, add after `last_processed_bitcoin_block`:

```rust
    pub limits: Option<Limits>,
```

- [ ] **Step 3: Parse `limits` in `DepositAccount::from_active_contract`**

In `DepositAccount::from_active_contract`, after the `last_processed_bitcoin_block` parsing, add:

```rust
        let limits = args
            .get("limits")
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    serde_json::from_value::<Limits>(v.clone()).ok()
                }
            });
```

And include `limits` in the `Ok(Self { ... })` return.

- [ ] **Step 4: Add `limits` to `DepositAccountStatus`**

In `DepositAccountStatus`, add after `last_processed_bitcoin_block`:

```rust
    pub limits: Option<Limits>,
```

- [ ] **Step 5: Add `limits` to `WithdrawAccount` struct**

In `WithdrawAccount`, add after `created_event_blob`:

```rust
    pub limits: Option<Limits>,
```

- [ ] **Step 6: Parse `limits` in `WithdrawAccount::from_active_contract`**

In `WithdrawAccount::from_active_contract`, after the `pending_balance` parsing, add:

```rust
        let limits = args
            .get("limits")
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    serde_json::from_value::<Limits>(v.clone()).ok()
                }
            });
```

And include `limits` in the `Ok(Self { ... })` return.

- [ ] **Step 7: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Compilation may fail in `mint.rs` (`get_deposit_account_status`) because `DepositAccountStatus` now needs `limits`. That will be fixed in Task 3.

- [ ] **Step 8: Commit**

```bash
git add src/mint_redeem/models.rs
git commit -m "feat: add Limits struct, check_limits helper, and limits field to account models"
```

---

### Task 2: Propagate `limits` through `mint.rs`

**Files:**
- Modify: `src/mint_redeem/mint.rs`

- [ ] **Step 1: Add `limits` to `DepositAccountStatus` construction in `get_deposit_account_status`**

In `get_deposit_account_status`, update the `Ok(DepositAccountStatus { ... })` block to include:

```rust
    Ok(DepositAccountStatus {
        contract_id: account.contract_id,
        owner: account.owner,
        operator: account.operator,
        registrar: account.registrar,
        bitcoin_address,
        last_processed_bitcoin_block: account.last_processed_bitcoin_block,
        limits: account.limits,
    })
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Success (or warnings only). The `limits` field is now propagated everywhere.

- [ ] **Step 3: Commit**

```bash
git add src/mint_redeem/mint.rs
git commit -m "feat: propagate limits field to DepositAccountStatus"
```

---

### Task 3: Add `credential_cids` to `CreateDepositAccountParams` and choice argument

**Files:**
- Modify: `src/mint_redeem/mint.rs`

- [ ] **Step 1: Add `credential_cids` field to `CreateDepositAccountParams`**

```rust
pub struct CreateDepositAccountParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub account_rules: AccountContractRuleSet,
    pub credential_cids: Vec<String>,
}
```

- [ ] **Step 2: Include `credentialCids` in the choice argument JSON**

In `create_deposit_account`, change the choice argument from:

```rust
    let choice_argument = json!({
        "owner": params.party
    });
```

to:

```rust
    let choice_argument = json!({
        "owner": params.party,
        "credentialCids": params.credential_cids
    });
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Success (may have warnings). Existing tests in `mint.rs` only call `list_deposit_accounts` (not `create_deposit_account`), so they won't break.

- [ ] **Step 4: Commit**

```bash
git add src/mint_redeem/mint.rs
git commit -m "feat: add credential_cids to CreateDepositAccountParams"
```

---

### Task 4: Add `credential_cids` to `CreateWithdrawAccountParams` and `SubmitWithdrawParams`

**Files:**
- Modify: `src/mint_redeem/redeem.rs`

- [ ] **Step 1: Add `credential_cids` to `CreateWithdrawAccountParams`**

```rust
pub struct CreateWithdrawAccountParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub account_rules_contract_id: String,
    pub account_rules_template_id: String,
    pub account_rules_created_event_blob: String,
    pub destination_btc_address: String,
    pub credential_cids: Vec<String>,
}
```

- [ ] **Step 2: Include `credentialCids` in `create_withdraw_account` choice argument**

In `create_withdraw_account`, change the choice argument from:

```rust
    let choice_argument = json!({
        "owner": params.party,
        "destinationBtcAddress": params.destination_btc_address
    });
```

to:

```rust
    let choice_argument = json!({
        "owner": params.party,
        "destinationBtcAddress": params.destination_btc_address,
        "credentialCids": params.credential_cids
    });
```

- [ ] **Step 3: Add `credential_cids` to `SubmitWithdrawParams`**

```rust
pub struct SubmitWithdrawParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub api_url: String,
    pub withdraw_account_contract_id: String,
    pub withdraw_account_template_id: String,
    pub withdraw_account_created_event_blob: String,
    pub amount: String,
    pub holding_contract_ids: Vec<String>,
    pub credential_cids: Option<Vec<String>>,
}
```

- [ ] **Step 4: Include `credentialCids` in `submit_withdraw` choice argument**

In `submit_withdraw`, the choice argument is built via manual format string. Add `credentialCids` to the format string. Change:

```rust
    let choice_argument_str = format!(
        r#"{{
            "tokens": {},
            "amount": "{}",
            "burnMintFactoryCid": "{}",
            "extraArgs": {}
        }}"#,
        serde_json::to_string(&params.holding_contract_ids).unwrap(),
        params.amount,
        token_contracts.burn_mint_factory.contract_id,
        serde_json::to_string(&extra_args).unwrap()
    );
```

to:

```rust
    let credential_cids_json = match &params.credential_cids {
        Some(cids) => serde_json::to_string(cids).unwrap(),
        None => "null".to_string(),
    };

    let choice_argument_str = format!(
        r#"{{
            "tokens": {},
            "amount": "{}",
            "burnMintFactoryCid": "{}",
            "extraArgs": {},
            "credentialCids": {}
        }}"#,
        serde_json::to_string(&params.holding_contract_ids).unwrap(),
        params.amount,
        token_contracts.burn_mint_factory.contract_id,
        serde_json::to_string(&extra_args).unwrap(),
        credential_cids_json
    );
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Success (may have warnings). Existing tests only call `list_withdraw_accounts` (not `create_withdraw_account` or `submit_withdraw`), so they won't break.

- [ ] **Step 6: Commit**

```bash
git add src/mint_redeem/redeem.rs
git commit -m "feat: add credential_cids to CreateWithdrawAccountParams and SubmitWithdrawParams"
```

---

### Task 5: Create `src/credentials.rs` module

**Files:**
- Create: `src/credentials.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add `pub mod credentials;` to `src/lib.rs`**

Add after the last existing `pub mod` line:

```rust
pub mod credentials;
```

- [ ] **Step 2: Create `src/credentials.rs` with constants, models, and param structs**

```rust
use common::submission;
use ledger::active_contracts;
use ledger::common::{TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter};
use ledger::ledger_end;
use ledger::models::JsActiveContract;
use ledger::submit;
use serde::{Deserialize, Serialize};
use serde_json::json;

// Template IDs for credential-related contracts
const CREDENTIAL_OFFER_TEMPLATE_ID: &str =
    "#utility-credential-app-v0:Utility.Credential.App.V0.Model.Offer:CredentialOffer";
const CREDENTIAL_TEMPLATE_ID: &str =
    "#utility-credential-v0:Utility.Credential.V0.Credential:Credential";
const USER_SERVICE_TEMPLATE_ID: &str =
    "#utility-credential-app-v0:Utility.Credential.App.V0.Service.User:UserService";

/// A claim within a credential (matches Daml Claim type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub subject: String,
    pub property: String,
    pub value: String,
}

/// A credential offer pending acceptance by the holder
#[derive(Debug, Clone)]
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

/// An active credential held by the user
#[derive(Debug, Clone)]
pub struct UserCredential {
    pub contract_id: String,
    pub template_id: String,
    pub issuer: String,
    pub holder: String,
    pub id: String,
    pub description: String,
    pub claims: Vec<Claim>,
}

/// Information about a UserService contract
#[derive(Debug, Clone)]
pub struct UserServiceInfo {
    pub contract_id: String,
    pub template_id: String,
    pub operator: String,
    pub user: String,
    pub dso: String,
}

/// Parameters for listing credential offers
pub struct ListCredentialOffersParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for listing credentials
pub struct ListCredentialsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for accepting a credential offer
pub struct AcceptCredentialOfferParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    pub user_service_contract_id: String,
    pub user_service_template_id: String,
    pub credential_offer_cid: String,
}

/// Parameters for finding a user's UserService contract
pub struct FindUserServiceParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Warnings about unused imports/types. No errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs src/credentials.rs
git commit -m "feat: scaffold credentials module with models and param structs"
```

---

### Task 6: Implement credential query and accept functions

**Files:**
- Modify: `src/credentials.rs`

- [ ] **Step 1: Implement helper to parse `CredentialOffer` from `JsActiveContract`**

Add to `src/credentials.rs`:

```rust
impl CredentialOffer {
    /// Parse a CredentialOffer from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();
        let template_id = contract.created_event.template_id.clone();
        let created_event_blob = contract.created_event.created_event_blob.clone();

        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let issuer = args
            .get("issuer")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'issuer' field")?
            .to_string();

        let holder = args
            .get("holder")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'holder' field")?
            .to_string();

        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'id' field")?
            .to_string();

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'description' field")?
            .to_string();

        let claims = args
            .get("claims")
            .and_then(|v| serde_json::from_value::<Vec<Claim>>(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            contract_id,
            template_id,
            created_event_blob,
            issuer,
            holder,
            id,
            description,
            claims,
        })
    }
}
```

- [ ] **Step 2: Implement helper to parse `UserCredential` from `JsActiveContract`**

Add to `src/credentials.rs`:

```rust
impl UserCredential {
    /// Parse a UserCredential from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();
        let template_id = contract.created_event.template_id.clone();

        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let issuer = args
            .get("issuer")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'issuer' field")?
            .to_string();

        let holder = args
            .get("holder")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'holder' field")?
            .to_string();

        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'id' field")?
            .to_string();

        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'description' field")?
            .to_string();

        let claims = args
            .get("claims")
            .and_then(|v| serde_json::from_value::<Vec<Claim>>(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            contract_id,
            template_id,
            issuer,
            holder,
            id,
            description,
            claims,
        })
    }
}
```

- [ ] **Step 3: Implement `list_credential_offers`**

Add to `src/credentials.rs`:

```rust
/// Fetch pending credential offers where the user is the holder
pub async fn list_credential_offers(
    params: ListCredentialOffersParams,
) -> Result<Vec<CredentialOffer>, String> {
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(CREDENTIAL_OFFER_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        });

    let contracts = active_contracts::get_by_party(active_contracts::Params {
        ledger_host: params.ledger_host,
        party: params.party.clone(),
        filter,
        access_token: params.access_token,
        ledger_end: ledger_end_response.offset,
        unknown_contract_entry_handler: None,
    })
    .await?;

    let offers: Vec<CredentialOffer> = contracts
        .iter()
        .filter_map(|c| {
            CredentialOffer::from_active_contract(c)
                .ok()
                .filter(|offer| offer.holder == params.party)
        })
        .collect();

    Ok(offers)
}
```

- [ ] **Step 4: Implement `list_credentials`**

Add to `src/credentials.rs`:

```rust
/// Fetch active credentials where the user is the holder
pub async fn list_credentials(
    params: ListCredentialsParams,
) -> Result<Vec<UserCredential>, String> {
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(CREDENTIAL_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        });

    let contracts = active_contracts::get_by_party(active_contracts::Params {
        ledger_host: params.ledger_host,
        party: params.party.clone(),
        filter,
        access_token: params.access_token,
        ledger_end: ledger_end_response.offset,
        unknown_contract_entry_handler: None,
    })
    .await?;

    let credentials: Vec<UserCredential> = contracts
        .iter()
        .filter_map(|c| {
            UserCredential::from_active_contract(c)
                .ok()
                .filter(|cred| cred.holder == params.party)
        })
        .collect();

    Ok(credentials)
}
```

- [ ] **Step 5: Implement `find_user_service`**

Add to `src/credentials.rs`:

```rust
/// Find the UserService contract for a given user party.
/// Filters where `user == party` to avoid returning the registrar's UserService.
pub async fn find_user_service(
    params: FindUserServiceParams,
) -> Result<UserServiceInfo, String> {
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(USER_SERVICE_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        });

    let contracts = active_contracts::get_by_party(active_contracts::Params {
        ledger_host: params.ledger_host,
        party: params.party.clone(),
        filter,
        access_token: params.access_token,
        ledger_end: ledger_end_response.offset,
        unknown_contract_entry_handler: None,
    })
    .await?;

    for contract in &contracts {
        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object());

        if let Some(args) = args {
            let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("");
            if user == params.party {
                let operator = args
                    .get("operator")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let dso = args
                    .get("dso")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                return Ok(UserServiceInfo {
                    contract_id: contract.created_event.contract_id.clone(),
                    template_id: contract.created_event.template_id.clone(),
                    operator,
                    user: user.to_string(),
                    dso,
                });
            }
        }
    }

    Err(format!(
        "No UserService contract found for party {}. The user must be onboarded to the Canton Network utility first.",
        params.party
    ))
}
```

- [ ] **Step 6: Implement `accept_credential_offer`**

Add to `src/credentials.rs`:

```rust
/// Accept a free credential offer by exercising UserService_AcceptFreeCredentialOffer.
/// Returns the created UserCredential.
pub async fn accept_credential_offer(
    params: AcceptCredentialOfferParams,
) -> Result<UserCredential, String> {
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    let choice_argument = json!({
        "credentialOfferCid": params.credential_offer_cid
    });

    let exercise_command = common::submission::ExerciseCommand {
        exercise_command: common::submission::ExerciseCommandData {
            template_id: params.user_service_template_id.clone(),
            contract_id: params.user_service_contract_id.clone(),
            choice: "UserService_AcceptFreeCredentialOffer".to_string(),
            choice_argument: common::submission::ChoiceArgumentsVariations::Generic(
                choice_argument,
            ),
        },
    };

    let submission_request = common::submission::Submission {
        act_as: vec![params.party.clone()],
        read_as: None,
        command_id,
        disclosed_contracts: vec![],
        commands: vec![common::submission::Command::ExerciseCommand(
            exercise_command,
        )],
        ..Default::default()
    };

    let response_raw = submit::wait_for_transaction_tree(submit::Params {
        ledger_host: params.ledger_host.clone(),
        access_token: params.access_token.clone(),
        request: submission_request,
    })
    .await?;

    // Parse response to extract the created Credential
    let response: serde_json::Value = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    let events_by_id = response["transactionTree"]["eventsById"]
        .as_object()
        .ok_or("Failed to find eventsById in transaction")?;

    for (_key, event) in events_by_id {
        if let Some(created_event) = event.get("CreatedTreeEvent") {
            let template_id = created_event["value"]["templateId"].as_str().unwrap_or("");

            if template_id.ends_with(":Utility.Credential.V0.Credential:Credential") {
                let created_event_value = &created_event["value"];
                let active_contract = JsActiveContract {
                    created_event: Box::new(ledger::models::CreatedEvent {
                        contract_id: created_event_value["contractId"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        template_id: template_id.to_string(),
                        create_argument: Some(Some(
                            created_event_value["createArgument"].clone(),
                        )),
                        created_event_blob: created_event_value["createdEventBlob"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        ..Default::default()
                    }),
                    reassignment_counter: 0,
                    synchronizer_id: String::new(),
                };
                return UserCredential::from_active_contract(&active_contract);
            }
        }
    }

    Err("No Credential was created in the transaction".to_string())
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo build --lib 2>&1 | head -20`

Expected: Success (possibly with warnings about unused imports in examples).

- [ ] **Step 8: Commit**

```bash
git add src/credentials.rs
git commit -m "feat: implement credential query, accept, and find_user_service functions"
```

---

### Task 7: Add credentials example

**Files:**
- Create: `examples/credentials.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add example entry to `Cargo.toml`**

Add after the last `[[example]]` block:

```toml
[[example]]
name = "credentials"
path = "examples/credentials.rs"
```

- [ ] **Step 2: Create `examples/credentials.rs`**

```rust
use cbtc::credentials::{
    AcceptCredentialOfferParams, FindUserServiceParams, ListCredentialOffersParams,
    ListCredentialsParams,
};
use keycloak::login::{PasswordParams, password, password_url};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Credential Example ===\n");

    // Step 1: Authenticate with Keycloak
    println!("Step 1: Authenticating with Keycloak...");
    let params = PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };
    let login_response = password(params).await?;
    println!("  Authenticated successfully\n");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    // Step 2: Check for existing credentials
    println!("Step 2: Checking for existing credentials...");
    let credentials = cbtc::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    // Filter for CBTC Minter credentials
    let minter_credentials: Vec<_> = credentials
        .iter()
        .filter(|c| {
            c.claims
                .iter()
                .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
        })
        .collect();

    if !minter_credentials.is_empty() {
        println!("  Found {} Minter credential(s):", minter_credentials.len());
        for cred in &minter_credentials {
            println!("    - ID: {}, Contract: {}", cred.id, cred.contract_id);
            println!("      Issuer: {}", cred.issuer);
            for claim in &cred.claims {
                println!(
                    "      Claim: {}.{} = {}",
                    claim.subject, claim.property, claim.value
                );
            }
        }
        println!();
        println!("=== Example Complete ===");
        println!("  Use credential CID {} in CBTC operations.", minter_credentials[0].contract_id);
        return Ok(());
    }

    println!("  No Minter credentials found.\n");

    // Step 3: Check for pending credential offers
    println!("Step 3: Checking for pending credential offers...");
    let offers = cbtc::credentials::list_credential_offers(ListCredentialOffersParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    if offers.is_empty() {
        println!("  No credential offers found.");
        println!("  The attestor network must offer you a credential before you can accept one.");
        println!("  Contact your CBTC operator to request credential issuance.");
        return Ok(());
    }

    println!("  Found {} credential offer(s):", offers.len());
    for offer in &offers {
        println!("    - ID: {}, Contract: {}", offer.id, offer.contract_id);
        println!("      Issuer: {}", offer.issuer);
        println!("      Description: {}", offer.description);
    }
    println!();

    // Step 4: Find UserService contract
    println!("Step 4: Finding UserService contract...");
    let user_service = cbtc::credentials::find_user_service(FindUserServiceParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    println!("  Found UserService: {}\n", user_service.contract_id);

    // Step 5: Accept the first offer
    let offer = &offers[0];
    println!(
        "Step 5: Accepting credential offer '{}'...",
        offer.id
    );
    let credential = cbtc::credentials::accept_credential_offer(AcceptCredentialOfferParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
        user_service_contract_id: user_service.contract_id.clone(),
        user_service_template_id: user_service.template_id.clone(),
        credential_offer_cid: offer.contract_id.clone(),
    })
    .await?;

    println!("  Credential accepted!");
    println!("    Contract ID: {}", credential.contract_id);
    println!("    ID: {}", credential.id);
    for claim in &credential.claims {
        println!(
            "    Claim: {}.{} = {}",
            claim.subject, claim.property, claim.value
        );
    }
    println!();

    println!("=== Example Complete ===");
    println!(
        "  Use credential CID {} in CBTC operations (e.g., create_deposit_account, submit_withdraw).",
        credential.contract_id
    );

    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build --example credentials 2>&1 | head -20`

Expected: Success.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml examples/credentials.rs
git commit -m "feat: add credentials example showing lifecycle (list, accept, use)"
```

---

### Task 8: Update `mint_cbtc_flow.rs` example with credentials

**Files:**
- Modify: `examples/mint_cbtc_flow.rs`

- [ ] **Step 1: Add credential imports**

Add at the top of the file after existing imports:

```rust
use cbtc::credentials::{ListCredentialsParams};
```

- [ ] **Step 2: Add credential fetching step before account creation**

After the "Step 3: Getting account contract rules" section and before "Step 4: Creating a new deposit account", add:

```rust
    // Step 3b: Fetch Minter credentials
    println!("Step 3b: Fetching Minter credentials...");
    let credentials = cbtc::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let credential_cids: Vec<String> = credentials
        .iter()
        .filter(|c| {
            c.claims
                .iter()
                .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
        })
        .map(|c| c.contract_id.clone())
        .collect();

    if credential_cids.is_empty() {
        return Err("No Minter credentials found. Run the credentials example first to accept a credential offer.".to_string());
    }
    println!("  Found {} Minter credential(s)\n", credential_cids.len());
```

- [ ] **Step 3: Add `credential_cids` to `CreateDepositAccountParams`**

Update the `create_deposit_account` call to include the new field:

```rust
    let deposit_account =
        cbtc::mint_redeem::mint::create_deposit_account(CreateDepositAccountParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            access_token: access_token.clone(),
            account_rules: account_rules.clone(),
            credential_cids,
        })
        .await?;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build --example mint_cbtc_flow 2>&1 | head -20`

Expected: Success.

- [ ] **Step 5: Commit**

```bash
git add examples/mint_cbtc_flow.rs
git commit -m "feat: update mint_cbtc_flow example to fetch and pass credentials"
```

---

### Task 9: Update `redeem_cbtc_flow.rs` example with credentials and limits

**Files:**
- Modify: `examples/redeem_cbtc_flow.rs`

- [ ] **Step 1: Read the current file to understand its structure**

Read `examples/redeem_cbtc_flow.rs` to understand the current flow and where to insert credential fetching and limit checks. The key points are:
- Where `CreateWithdrawAccountParams` is constructed
- Where `SubmitWithdrawParams` is constructed
- Where the withdraw amount is defined

- [ ] **Step 2: Add credential imports**

Add at the top after existing imports:

```rust
use cbtc::credentials::ListCredentialsParams;
use cbtc::mint_redeem::models::check_limits;
```

- [ ] **Step 3: Add credential fetching before account creation**

Before `create_withdraw_account` is called (before the `if !accounts.is_empty()` block around line 125), add credential fetching (same pattern as Task 8):

```rust
    // Fetch Minter credentials
    println!("Fetching Minter credentials...");
    let credentials = cbtc::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let credential_cids: Vec<String> = credentials
        .iter()
        .filter(|c| {
            c.claims
                .iter()
                .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
        })
        .map(|c| c.contract_id.clone())
        .collect();

    if credential_cids.is_empty() {
        return Err("No Minter credentials found. Run the credentials example first.".to_string());
    }
    println!("  Found {} Minter credential(s)\n", credential_cids.len());
```

- [ ] **Step 4: Add `credential_cids` to `CreateWithdrawAccountParams`**

In the `create_withdraw_account` call (around line 137), add `credential_cids: credential_cids.clone(),` to the struct. The full updated call:

```rust
        let withdraw_account =
            cbtc::mint_redeem::redeem::create_withdraw_account(CreateWithdrawAccountParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
                access_token: access_token.clone(),
                account_rules_contract_id: account_rules.wa_rules.contract_id.clone(),
                account_rules_template_id: account_rules.wa_rules.template_id.clone(),
                account_rules_created_event_blob: account_rules.wa_rules.created_event_blob.clone(),
                destination_btc_address: destination_btc_address.clone(),
                credential_cids: credential_cids.clone(),
            })
            .await?;
```

- [ ] **Step 5: Add limit check before `submit_withdraw`**

Before the `submit_withdraw` call (around line 214), after the existing balance check and holding selection, add:

```rust
    // Pre-check limits before submitting
    check_limits("Withdraw", withdraw_amount_f64, &withdraw_account.limits)?;
    println!("  Limit check passed");
```

Note: `withdraw_amount_f64` is already defined earlier in the file (line 180).

- [ ] **Step 6: Add `credential_cids` to `SubmitWithdrawParams`**

In the `submit_withdraw` call (around line 214), add `credential_cids: Some(credential_cids),` to the struct. The full updated call:

```rust
    let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(SubmitWithdrawParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        access_token: access_token.clone(),
        api_url: api_url.clone(),
        withdraw_account_contract_id: withdraw_account.contract_id.clone(),
        withdraw_account_template_id: withdraw_account.template_id.clone(),
        withdraw_account_created_event_blob: withdraw_account.created_event_blob.clone(),
        amount: withdraw_amount.to_string(),
        holding_contract_ids: selected_holdings,
        credential_cids: Some(credential_cids),
    })
    .await?;
```

- [ ] **Step 7: Verify all examples compile**

Run: `cargo build --examples 2>&1 | head -30`

Expected: All examples compile successfully.

- [ ] **Step 8: Commit**

```bash
git add examples/redeem_cbtc_flow.rs
git commit -m "feat: update redeem_cbtc_flow example with credentials and limit pre-check"
```

---

### Task 10: Final verification and cleanup

**Files:**
- All modified files

- [ ] **Step 1: Full build**

Run: `cargo build --release 2>&1 | tail -5`

Expected: Compiles successfully.

- [ ] **Step 2: Build all examples**

Run: `cargo build --examples --release 2>&1 | tail -5`

Expected: All examples compile.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --all-targets --all-features 2>&1 | tail -20`

Expected: No errors. Warnings are acceptable but should be reviewed.

- [ ] **Step 4: Run formatter**

Run: `cargo fmt --all`

- [ ] **Step 5: Commit any formatting changes**

```bash
git add -A
git commit -m "chore: format code"
```

- [ ] **Step 6: Push**

```bash
git push
```
