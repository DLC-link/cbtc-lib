# Integration Test Extension: Account Creation, Deposits & Withdrawals

## Summary

Extend `examples/integration_test.rs` to cover deposit account creation, withdraw account creation, credential validation, faucet deposits (optional), and full withdrawal flow -- in addition to the existing transfer round-trip.

## New Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BITSAFE_API_URL` | Yes | -- | Bitsafe/attestor API URL (already used elsewhere in codebase) |
| `DESTINATION_BTC_ADDRESS` | No | `tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx` | BTC address for withdraw account |
| `WITHDRAW_AMOUNT` | No | Value of `TRANSFER_AMOUNT` | Amount to withdraw/burn. Must be <= sender's starting balance. |
| `FAUCET_URL` | No | -- | If set, enables faucet deposit steps |
| `FAUCET_NETWORK` | No | `devnet` | Network name for faucet API |

Note: The original plan proposed `ATTESTOR_URL` and `CANTON_NETWORK` but these don't correspond to any actual API parameters. All attestor/Bitsafe API functions take a single `api_url` argument.

## Step Flow

### Without faucet (18 steps)

| Step | Description | New? | Notes |
|------|-------------|------|-------|
| 1 | Check sender balance | | Existing |
| 2 | Check receiver balance | | Existing |
| **3** | **Fetch Minter credentials (sender)** | **NEW** | Fails early if no Minter credentials found |
| **4** | **Fetch account rules from Bitsafe API** | **NEW** | Single call, result reused in steps 5 and 7 |
| **5** | **Create deposit account (sender)** | **NEW** | Always creates new (no cleanup API exists) |
| **6** | **Get Bitcoin address for deposit account** | **NEW** | Verifies attestor can derive a BTC address |
| **7** | **Create withdraw account (sender)** | **NEW** | Uses `wa_rules` from account rules |
| 8 | Send CBTC sender -> receiver | | Existing (was step 3) |
| 9 | List outgoing offers (sender) | | Existing (was step 4) |
| 10 | List incoming offers (receiver) | | Existing (was step 5) |
| 11 | Accept transfers (receiver) | | Existing (was step 6) |
| 12 | Check receiver balance | | Existing (was step 7) |
| 13 | Return CBTC receiver -> sender | | Existing (was step 8) |
| 14 | Accept transfers (sender) | | Existing (was step 9) |
| 15 | Check sender balance | **MODIFIED** | Existing (was step 10). Now also stores balance in `pre_withdraw_balance` for step 17 comparison. |
| **16** | **Submit withdrawal (sender)** | **NEW** | Burns CBTC, includes limit pre-check |
| **17** | **Check sender balance (post-withdraw)** | **NEW** | Verifies tokens were burned |
| 18 | Consolidate UTXOs (sender) | | Existing (was step 11) |

### With faucet (+3 = 21 steps)

Insert after step 7:

| Step | Description |
|------|-------------|
| **8** | **Request CBTC from faucet** |
| **9** | **Poll for incoming faucet transfer** |
| **10** | **Accept faucet transfer** |

Steps 8-18 from the no-faucet flow shift to 11-21.

## Implementation Details

### Dynamic step count

Replace `const TOTAL_STEPS: usize = 11` with:

```rust
let faucet_url = env::var("FAUCET_URL").ok();
let base_steps: usize = 18;
let total_steps = base_steps + if faucet_url.is_some() { 3 } else { 0 };
```

Changes needed to replace the `TOTAL_STEPS` const:

1. **`print_step`**: Add a `total: usize` parameter. Signature becomes `fn print_step(step: usize, total: usize, description: &str)`. Replace `TOTAL_STEPS` with `total` in the format string.
2. **`run_step!` macro**: Replace all references to `TOTAL_STEPS` with the local `total_steps` variable. The macro captures locals by name, so it will pick up `total_steps` from `main()` scope automatically. Both the `print_step(step, total_steps, $desc)` call and the `print_summary(passed, total_steps, ...)` call in the error path need updating.
3. **`print_summary`**: Already takes `total: usize` as a parameter -- no change needed.
4. **Final `print_summary` call** at the end of `main()`: Change `TOTAL_STEPS` to `total_steps`.

### New env var loading (in `main()`)

```rust
let bitsafe_api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");
let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
    .unwrap_or_else(|_| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.clone());
let faucet_url = env::var("FAUCET_URL").ok();
let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());
```

### Early validation (before steps begin)

After loading env vars and checking `sender.party_id != receiver.party_id` (existing check), add:

```rust
let withdraw_amount_f64: f64 = withdraw_amount.parse()
    .expect("WITHDRAW_AMOUNT must be a valid number");
```

This validates the format early. The actual balance sufficiency check happens at step 16 when holdings are known.

### Step 3: Fetch Minter credentials

Note: `minter_credential_cids` is declared as `let mut minter_credential_cids: Vec<String> = Vec::new();` in `main()` scope (see Variable Scoping). The async block assigns to it directly — no `let` binding, which would shadow the outer variable.

```rust
let token = authenticate(&sender).await?;
let credentials = cbtc::credentials::list_credentials(cbtc::credentials::ListCredentialsParams {
    ledger_host: sender.ledger_host.clone(),
    party: sender.party_id.clone(),
    access_token: token,
}).await?;

minter_credential_cids = credentials
    .iter()
    .filter(|c| c.claims.iter().any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter"))
    .map(|c| c.contract_id.clone())
    .collect();

if minter_credential_cids.is_empty() {
    return Err("No Minter credentials found for sender party".to_string());
}
Ok::<String, String>(format!("({} Minter credentials)", minter_credential_cids.len()))
```

### Step 4: Fetch account rules

`account_rules` is `Option<AccountContractRuleSet>` in `main()` scope. Assign with `Some(...)`:

```rust
account_rules = Some(cbtc::mint_redeem::attestor::get_account_contract_rules(
    &bitsafe_api_url
).await?);
Ok::<String, String>(format!("(da_rules + wa_rules)"))
```

### Step 5: Create deposit account

Unwrap `account_rules` (guaranteed `Some` after step 4). Assign result to outer `deposit_account`:

```rust
let token = authenticate(&sender).await?;
let rules = account_rules.as_ref().unwrap();
let account = cbtc::mint_redeem::mint::create_deposit_account(
    cbtc::mint_redeem::mint::CreateDepositAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules: rules.clone(),
        credential_cids: minter_credential_cids.clone(),
    }
).await?;
deposit_account = Some(account.clone());
Ok::<String, String>(format!("(owner={}, cid={})", account.owner, &account.contract_id[..16]))
```

### Step 6: Get Bitcoin address

Unwrap `deposit_account` (guaranteed `Some` after step 5):

```rust
let da = deposit_account.as_ref().unwrap();
let btc_address = cbtc::mint_redeem::mint::get_bitcoin_address(
    cbtc::mint_redeem::mint::GetBitcoinAddressParams {
        api_url: bitsafe_api_url.clone(),
        account_id: da.account_id().to_string(),
    }
).await?;
Ok::<String, String>(format!("({})", btc_address))
```

### Step 7: Create withdraw account

Unwrap `account_rules`. Assign result to outer `withdraw_account`:

```rust
let token = authenticate(&sender).await?;
let rules = account_rules.as_ref().unwrap();
let account = cbtc::mint_redeem::redeem::create_withdraw_account(
    cbtc::mint_redeem::redeem::CreateWithdrawAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules_contract_id: rules.wa_rules.contract_id.clone(),
        account_rules_template_id: rules.wa_rules.template_id.clone(),
        account_rules_created_event_blob: rules.wa_rules.created_event_blob.clone(),
        destination_btc_address: destination_btc_address.clone(),
        credential_cids: minter_credential_cids.clone(),
    }
).await?;
withdraw_account = Some(account.clone());
Ok::<String, String>(format!("(dest={})", account.destination_btc_address))
```

### Faucet steps (conditional, steps 8-10 when enabled)

Only run if `FAUCET_URL` is set. The code structure is a single `if let Some(ref faucet_url) = faucet_url { ... }` block containing three `run_step!` calls. Because `run_step!` auto-increments `step` via `step += 1`, the step counter adjusts automatically — no manual offset logic is needed. When faucet is disabled, these three `run_step!` calls are simply skipped and subsequent steps get lower numbers, which is correct since `total_steps` was computed without the faucet bonus.

```rust
if let Some(ref faucet_url) = faucet_url {
    // step 8: request from faucet
    run_step!("Request CBTC from faucet", async { ... });
    // step 9: poll for incoming
    run_step!("Poll for faucet transfer", async { ... });
    // step 10: accept
    run_step!("Accept faucet transfer", async { ... });
}
```

**Step 8: Request CBTC from faucet**

Captures baseline incoming transfer count in `pre_faucet_count` (outer `main()` variable) *before* making the faucet request. This is needed by step 9 to detect the new transfer.

```rust
// Capture baseline incoming count before faucet request
let token = authenticate(&sender).await?;
let pre_faucet_incoming = cbtc::utils::fetch_incoming_transfers(
    sender.ledger_host.clone(),
    sender.party_id.clone(),
    token,
).await?;
pre_faucet_count = pre_faucet_incoming.len();

// Make faucet request
let client = reqwest::Client::new();
let resp = client.post(format!("{}/api/faucet", faucet_url))
    .json(&serde_json::json!({
        "network": faucet_network,
        "recipient_party": sender.party_id,
        "amount": amount,
    }))
    .send()
    .await
    .map_err(|e| format!("Faucet request failed: {}", e))?;

if !resp.status().is_success() {
    return Err(format!("Faucet returned status: {}", resp.status()));
}
Ok::<String, String>(format!("(requested {} CBTC, {} existing incoming)", amount, pre_faucet_count))
```

**Step 9: Poll for incoming faucet transfer**

Fixed 3-second interval, 30-second timeout (10 attempts max). Reads `pre_faucet_count` from outer scope (set in step 8). Polls until incoming transfer count exceeds the baseline — this avoids false positives from pre-existing transfers.

```rust
let mut attempts = 0;
let max_attempts = 10;
let poll_interval = std::time::Duration::from_secs(3);

loop {
    let token = authenticate(&sender).await?;
    let incoming = cbtc::utils::fetch_incoming_transfers(
        sender.ledger_host.clone(),
        sender.party_id.clone(),
        token,
    ).await?;

    if incoming.len() > pre_faucet_count { break; }

    attempts += 1;
    if attempts >= max_attempts {
        return Err("No incoming faucet transfer after 30s".to_string());
    }
    tokio::time::sleep(poll_interval).await;
}
Ok::<String, String>(format!("(found after {}s)", attempts * 3))
```

**Step 10: Accept faucet transfer**

Use `cbtc::accept::accept_all()` (same pattern as existing step 11/accept transfers).

### Step 15: Check sender balance (modified)

The existing step 15 (was step 10) checks sender balance. Modify it to also store the balance for post-withdraw comparison:

```rust
let (balance, utxos) = check_balance(&sender).await?;
pre_withdraw_balance = balance;
Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
```

### Step 16: Submit withdrawal

Unwrap `withdraw_account` (guaranteed `Some` after step 7). Uses `withdraw_amount_f64` from early validation.

```rust
let token = authenticate(&sender).await?;
let wa = withdraw_account.as_ref().unwrap();

// List holdings and select enough to cover withdraw_amount
let holdings = cbtc::mint_redeem::redeem::list_holdings(
    cbtc::mint_redeem::redeem::ListHoldingsParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        access_token: token.clone(),
    }
).await?;

let cbtc_holdings: Vec<_> = holdings.iter()
    .filter(|h| h.instrument_id == "CBTC")
    .collect();

// Greedy select holdings to cover withdraw_amount
let mut selected = Vec::new();
let mut selected_total = 0.0;
for h in &cbtc_holdings {
    let amt = h.amount.parse::<f64>().unwrap_or(0.0);
    selected.push(h.contract_id.clone());
    selected_total += amt;
    if selected_total >= withdraw_amount_f64 { break; }
}
if selected_total < withdraw_amount_f64 {
    return Err(format!("Insufficient holdings: have {}, need {}", selected_total, withdraw_amount));
}

// Pre-check limits
cbtc::mint_redeem::models::check_limits("Withdraw", withdraw_amount_f64, &wa.limits)?;

// Submit
let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(
    cbtc::mint_redeem::redeem::SubmitWithdrawParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        api_url: bitsafe_api_url.clone(),
        withdraw_account_contract_id: wa.contract_id.clone(),
        withdraw_account_template_id: wa.template_id.clone(),
        withdraw_account_created_event_blob: wa.created_event_blob.clone(),
        amount: withdraw_amount.clone(),
        holding_contract_ids: selected,
        credential_cids: Some(minter_credential_cids.clone()),
    }
).await?;
Ok::<String, String>(format!("(burned {} CBTC, pending={})", withdraw_amount, updated_account.pending_balance))
```

### Step 17: Check sender balance (post-withdraw)

Compare against `pre_withdraw_balance` (set in step 15). Fail if balance didn't decrease:

```rust
let (balance, utxos) = check_balance(&sender).await?;
if balance >= pre_withdraw_balance {
    return Err(format!(
        "Balance did not decrease after withdrawal: was {:.8}, now {:.8}",
        pre_withdraw_balance, balance
    ));
}
Ok::<String, String>(format!(
    "({:.8} CBTC, {} UTXOs, burned ~{:.8})",
    balance, utxos, pre_withdraw_balance - balance
))
```

### Step 18: Consolidate (unchanged logic)

Remains the last step. Uses `total_steps` instead of `TOTAL_STEPS`.

## Variable Scoping

Values that need to be shared across `run_step!` closures must be declared in `main()` scope. For values produced inside a `run_step!` block, use `Option<T>` initialized to `None` and assigned inside the block:

```rust
let mut minter_credential_cids: Vec<String> = Vec::new();
let mut account_rules = None;           // Option<AccountContractRuleSet>
let mut deposit_account = None;         // Option<DepositAccount>
let mut withdraw_account = None;        // Option<WithdrawAccount>
let mut pre_faucet_count: usize = 0;    // baseline incoming transfer count before faucet request
let mut pre_withdraw_balance: f64 = 0.0; // sender balance captured in step 15 for post-withdraw assertion
```

Unwrap with `.expect()` or `.clone().unwrap()` in later steps -- safe because step ordering guarantees they're set.

Note: `pre_faucet_count` is set inside the step 8 async block and read inside the step 9 async block. This works because the `run_step!` macro's async closures capture mutable locals from `main()` scope (same pattern as the existing `sender_has_pending_offer` flag).

## Account Cleanup Note

No cleanup API exists for deposit/withdraw accounts. These are persistent Canton contracts. Accounts created during the test will remain -- this is harmless. Each test run creates new accounts. If the test fails mid-way (e.g., step 5 succeeds but step 7 fails), orphaned accounts are left behind -- this is also harmless. The existing `run_step!` error handler only cleans up pending transfer offers, which remains correct. Document this in the test's doc comment.

## Doc Comment Update

The file's top-level doc comment (lines 1-23) lists required and optional env vars. Update it to include:

- **Required**: `BITSAFE_API_URL`
- **Optional**: `DESTINATION_BTC_ADDRESS`, `WITHDRAW_AMOUNT`, `FAUCET_URL`, `FAUCET_NETWORK`
- **Note**: Add a line about persistent deposit/withdraw accounts created during the test.

## What's NOT Changing

- `PartyConfig` struct, `load_sender_config()`, `load_receiver_config()`, `authenticate()`, `check_balance()`
- `print_header`, `print_ok`, `print_fail`, `print_skip` functions
- `cleanup_sender_offers` function
- The `run_step!` macro pattern (inline async closures)

## Verification

1. `cargo check --example integration_test` -- compiles
2. Run against devnet with all env vars including `BITSAFE_API_URL`
3. Run without `FAUCET_URL` to verify faucet steps are skipped and step count = 18
4. Run with `FAUCET_URL` to verify full flow including faucet deposit and step count = 21
