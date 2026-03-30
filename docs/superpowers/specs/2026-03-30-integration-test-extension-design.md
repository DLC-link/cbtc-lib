# Integration Test Extension: Account Creation, Deposits & Withdrawals

## Summary

Extend `examples/integration_test.rs` to cover deposit account creation, withdraw account creation, credential validation, faucet deposits (optional), and full withdrawal flow -- in addition to the existing transfer round-trip.

## New Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BITSAFE_API_URL` | Yes | -- | Bitsafe/attestor API URL (already used elsewhere in codebase) |
| `DESTINATION_BTC_ADDRESS` | No | `tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx` | BTC address for withdraw account |
| `WITHDRAW_AMOUNT` | No | Value of `TRANSFER_AMOUNT` | Amount to withdraw/burn |
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
| 15 | Check sender balance | | Existing (was step 10) |
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

Update `print_step`, `print_summary`, and the `run_step!` macro to use `total_steps` variable instead of `TOTAL_STEPS` const.

### New env var loading (in `main()`)

```rust
let bitsafe_api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");
let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
    .unwrap_or_else(|_| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.clone());
let faucet_url = env::var("FAUCET_URL").ok();
let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());
```

### Step 3: Fetch Minter credentials

```rust
let token = authenticate(&sender).await?;
let credentials = cbtc::credentials::list_credentials(cbtc::credentials::ListCredentialsParams {
    ledger_host: sender.ledger_host.clone(),
    party: sender.party_id.clone(),
    access_token: token,
}).await?;

let minter_credential_cids: Vec<String> = credentials
    .iter()
    .filter(|c| c.claims.iter().any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter"))
    .map(|c| c.contract_id.clone())
    .collect();

if minter_credential_cids.is_empty() {
    return Err("No Minter credentials found for sender party".to_string());
}
```

`minter_credential_cids` is stored in `main()` scope for reuse in steps 5, 7, and 16.

### Step 4: Fetch account rules

```rust
let account_rules = cbtc::mint_redeem::attestor::get_account_contract_rules(
    &bitsafe_api_url
).await?;
```

Single argument. Result stored in `main()` scope for steps 5 and 7.

### Step 5: Create deposit account

```rust
let token = authenticate(&sender).await?;
let deposit_account = cbtc::mint_redeem::mint::create_deposit_account(
    cbtc::mint_redeem::mint::CreateDepositAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules: account_rules.clone(),
        credential_cids: minter_credential_cids.clone(),
    }
).await?;
```

Verify: `deposit_account.owner == sender.party_id` and `contract_id` is non-empty.

### Step 6: Get Bitcoin address

```rust
let btc_address = cbtc::mint_redeem::mint::get_bitcoin_address(
    cbtc::mint_redeem::mint::GetBitcoinAddressParams {
        api_url: bitsafe_api_url.clone(),
        account_id: deposit_account.account_id().to_string(),
    }
).await?;
```

Print the BTC address in the OK output.

### Step 7: Create withdraw account

```rust
let token = authenticate(&sender).await?;
let withdraw_account = cbtc::mint_redeem::redeem::create_withdraw_account(
    cbtc::mint_redeem::redeem::CreateWithdrawAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules_contract_id: account_rules.wa_rules.contract_id.clone(),
        account_rules_template_id: account_rules.wa_rules.template_id.clone(),
        account_rules_created_event_blob: account_rules.wa_rules.created_event_blob.clone(),
        destination_btc_address: destination_btc_address.clone(),
        credential_cids: minter_credential_cids.clone(),
    }
).await?;
```

### Faucet steps (conditional, steps 8-10 when enabled)

Only run if `FAUCET_URL` is set.

**Step 8: Request CBTC from faucet**

```rust
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
```

**Step 9: Poll for incoming faucet transfer**

Fixed 3-second interval, 30-second timeout (10 attempts max):

```rust
let mut attempts = 0;
let max_attempts = 10;
let poll_interval = std::time::Duration::from_secs(3);
let mut incoming = Vec::new();

loop {
    let token = authenticate(&sender).await?;
    incoming = cbtc::utils::fetch_incoming_transfers(
        sender.ledger_host.clone(),
        sender.party_id.clone(),
        token,
    ).await?;

    if !incoming.is_empty() { break; }

    attempts += 1;
    if attempts >= max_attempts {
        return Err("No incoming faucet transfer after 30s".to_string());
    }
    tokio::time::sleep(poll_interval).await;
}
```

**Step 10: Accept faucet transfer**

Use `cbtc::accept::accept_all()` (same pattern as existing step 11/accept transfers).

### Step 16: Submit withdrawal

```rust
let token = authenticate(&sender).await?;

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
let withdraw_amount_f64: f64 = withdraw_amount.parse()
    .map_err(|e| format!("Invalid WITHDRAW_AMOUNT: {}", e))?;
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
cbtc::mint_redeem::models::check_limits("Withdraw", withdraw_amount_f64, &withdraw_account.limits)?;

// Submit
let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(
    cbtc::mint_redeem::redeem::SubmitWithdrawParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        api_url: bitsafe_api_url.clone(),
        withdraw_account_contract_id: withdraw_account.contract_id.clone(),
        withdraw_account_template_id: withdraw_account.template_id.clone(),
        withdraw_account_created_event_blob: withdraw_account.created_event_blob.clone(),
        amount: withdraw_amount.clone(),
        holding_contract_ids: selected,
        credential_cids: Some(minter_credential_cids.clone()),
    }
).await?;
```

### Step 17: Check sender balance (post-withdraw)

Same as existing `check_balance()` call. Print balance to confirm decrease.

### Step 18: Consolidate (unchanged logic)

Remains the last step. Uses `total_steps` instead of `TOTAL_STEPS`.

## Variable Scoping

Values that need to be shared across `run_step!` closures must be declared in `main()` scope. For values produced inside a `run_step!` block, use `Option<T>` initialized to `None` and assigned inside the block:

```rust
let mut minter_credential_cids: Vec<String> = Vec::new();
let mut account_rules = None;       // Option<AccountContractRuleSet>
let mut deposit_account = None;     // Option<DepositAccount>
let mut withdraw_account = None;    // Option<WithdrawAccount>
```

Unwrap with `.expect()` or `.clone().unwrap()` in later steps -- safe because step ordering guarantees they're set.

## Account Cleanup Note

No cleanup API exists for deposit/withdraw accounts. These are persistent Canton contracts. Accounts created during the test will remain -- this is harmless. Each test run creates new accounts. Document this in the test's doc comment.

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
