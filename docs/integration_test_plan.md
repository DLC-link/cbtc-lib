# Extend Integration Test: Account Creation, Deposits & Withdrawals

## Context

The current `examples/integration_test.rs` covers a transfer round-trip (send -> verify -> accept -> return -> consolidate) between two parties. We want to extend it to also cover:
1. **Deposit account creation** -- verify Canton + attestor flow
2. **Withdraw account creation** -- verify withdraw account setup
3. **Faucet deposit** (optional) -- request CBTC from faucet, accept it
4. **Full withdrawal** -- burn CBTC and verify withdraw request

## New Environment Variables

```
# Required for new steps:
ATTESTOR_URL             -- Attestor service URL
CANTON_NETWORK           -- Chain identifier (e.g., "canton-devnet")

# Optional:
DESTINATION_BTC_ADDRESS  -- BTC address for withdraw account (default: test address)
WITHDRAW_AMOUNT          -- Amount to withdraw (default: same as TRANSFER_AMOUNT)
FAUCET_URL               -- If set, run faucet deposit test (e.g., "https://faucet.example.com")
FAUCET_NETWORK           -- Network name for faucet API (default: "devnet")
```

## Account Cleanup Note

**No cleanup API exists** in cbtc-lib for deposit/withdraw accounts. These are persistent Canton contracts. The DAML templates don't expose archive choices. Accounts created during the test will remain -- this is harmless but worth noting. We'll document this in the test output.

## Proposed Step Flow

Dynamic step count based on whether `FAUCET_URL` is set.

### Without faucet (15 steps):

| Step | Description | New? |
|------|-------------|------|
| 1 | Check sender balance | |
| 2 | Check receiver balance | |
| **3** | **Create deposit account (sender)** | NEW |
| **4** | **Get Bitcoin address for deposit account** | NEW |
| **5** | **Create withdraw account (sender)** | NEW |
| 6 | Send CBTC sender -> receiver | |
| 7 | List outgoing offers (sender) | |
| 8 | List incoming offers (receiver) | |
| 9 | Accept transfers (receiver) | |
| 10 | Check receiver balance | |
| 11 | Return CBTC receiver -> sender | |
| 12 | Accept transfers (sender) | |
| 13 | Check sender balance | |
| **14** | **Submit withdrawal (sender)** | NEW |
| 15 | Consolidate UTXOs (sender) | |

### With faucet (+3 steps = 18 total):

Insert after step 5:

| Step | Description |
|------|-------------|
| **6** | **Request CBTC from faucet** |
| **7** | **List incoming offers (faucet transfer)** |
| **8** | **Accept faucet transfer** |

Everything else shifts by 3.

## Implementation Details

### File to modify
`examples/integration_test.rs`

### 1. Dynamic step count

Replace `const TOTAL_STEPS: usize = 11` with a computed value:

```rust
let faucet_url = env::var("FAUCET_URL").ok();
let base_steps: usize = 15;
let faucet_steps: usize = if faucet_url.is_some() { 3 } else { 0 };
let total_steps = base_steps + faucet_steps;
```

Update `print_step`, `print_summary`, and the `run_step!` macro to use `total_steps` variable instead of `TOTAL_STEPS` const.

### 2. New env var loading (in `main()`)

```rust
let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
let canton_network = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");
let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
    .unwrap_or_else(|_| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.clone());
let faucet_url = env::var("FAUCET_URL").ok();
let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());
```

### 3. Step: Create deposit account (sender)

Uses functions from `src/mint_redeem/attestor.rs` and `src/mint_redeem/mint.rs`:

```rust
// Fetch account rules (needed for both deposit and withdraw account creation)
let account_rules = cbtc::mint_redeem::attestor::get_account_contract_rules(
    &attestor_url, &canton_network
).await?;

// Create deposit account
let token = authenticate(&sender).await?;
let deposit_account = cbtc::mint_redeem::mint::create_deposit_account(
    cbtc::mint_redeem::mint::CreateDepositAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules: account_rules.clone(),
    }
).await?;
```

### 4. Step: Get Bitcoin address

```rust
let btc_address = cbtc::mint_redeem::mint::get_bitcoin_address(
    cbtc::mint_redeem::mint::GetBitcoinAddressParams {
        attestor_url: attestor_url.clone(),
        account_id: deposit_account.account_id().to_string(),
        chain: canton_network.clone(),
    }
).await?;
// Print the BTC address in OK output
```

### 5. Step: Create withdraw account (sender)

Uses `src/mint_redeem/redeem.rs`. Needs the withdraw rules portion from account_rules:

```rust
let token = authenticate(&sender).await?;
let withdraw_account = cbtc::mint_redeem::redeem::create_withdraw_account(
    cbtc::mint_redeem::redeem::CreateWithdrawAccountParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        account_rules_contract_id: account_rules.withdraw.contract_id.clone(),
        account_rules_template_id: account_rules.withdraw.template_id.clone(),
        account_rules_created_event_blob: account_rules.withdraw.created_event_blob.clone(),
        destination_btc_address: destination_btc_address.clone(),
    }
).await?;
```

### 6. Faucet steps (conditional)

Only run if `FAUCET_URL` is set. Uses `reqwest` to call faucet API.

**Step A: Request CBTC from faucet**

The faucet is a separate service (`cbtc-faucet`) with this API:

```
POST {faucet_url}/api/faucet
Content-Type: application/json

{
  "network": "devnet",
  "recipient_party": "<sender.party_id>",
  "amount": "<amount>"
}

Response: { "success": true, "message": "...", "network": "...", "recipient_party": "...", "amount": "..." }
```

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
```

**Step B: List incoming offers (wait for faucet transfer)**

Poll `cbtc::utils::fetch_incoming_transfers()` with brief retry (up to ~30s) since the faucet transfer is async.

**Step C: Accept faucet transfer**

Use `cbtc::accept::accept_all()` to accept the incoming transfer from faucet.

### 7. Step: Submit withdrawal (sender)

```rust
let token = authenticate(&sender).await?;

// List holdings to find contract IDs to burn
let holdings = cbtc::mint_redeem::redeem::list_holdings(
    cbtc::mint_redeem::redeem::ListHoldingsParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        access_token: token.clone(),
    }
).await?;

// Get token standard contracts from attestor (needed for submit_withdraw)
let token_contracts = cbtc::mint_redeem::attestor::get_token_standard_contracts(
    &attestor_url, &canton_network
).await?;

// Select holdings to cover withdraw_amount, submit withdrawal
let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(
    cbtc::mint_redeem::redeem::SubmitWithdrawParams {
        ledger_host: sender.ledger_host.clone(),
        party: sender.party_id.clone(),
        user_name: sender.keycloak_username.clone(),
        access_token: token,
        attestor_url: attestor_url.clone(),
        chain: canton_network.clone(),
        withdraw_account_contract_id: withdraw_account.contract_id.clone(),
        withdraw_account_template_id: withdraw_account.template_id.clone(),
        withdraw_account_created_event_blob: withdraw_account.created_event_blob.clone(),
        amount: withdraw_amount.clone(),
        holding_contract_ids: holding_cids, // selected from holdings
    }
).await?;
```

### 8. Consolidate remains last step (unchanged logic)

## Verification

1. `cargo check --example integration_test` -- compiles
2. Run against devnet with all env vars including `ATTESTOR_URL` and `CANTON_NETWORK`
3. Run without `FAUCET_URL` to verify faucet steps are skipped and step count is correct
4. Run with `FAUCET_URL` to verify full flow including faucet deposit
