# Integration Test Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `examples/integration_test.rs` to cover deposit/withdraw account creation, credential validation, optional faucet deposits, and full withdrawal flow alongside the existing transfer round-trip.

**Architecture:** Single-file extension of the existing integration test. New steps are inserted as inline `run_step!` async closures matching the existing pattern. Dynamic step count replaces the `TOTAL_STEPS` const. Shared state across steps uses `mut` variables in `main()` scope.

**Tech Stack:** Rust, tokio, cbtc (local crate), keycloak (local crate), reqwest, serde_json, chrono

**Spec:** `docs/superpowers/specs/2026-03-30-integration-test-extension-design.md`

---

### File Map

- Modify: `examples/integration_test.rs` — the only file changed

---

### Task 1: Replace TOTAL_STEPS const with dynamic step count

**Files:**
- Modify: `examples/integration_test.rs:27` (const), `:110-118` (print_step), `:193-215` (run_step! macro), `:374-405` (consolidate step), `:407` (final print_summary)

- [ ] **Step 1: Remove `const TOTAL_STEPS` and update `print_step` signature**

Replace line 27:
```rust
const TOTAL_STEPS: usize = 11;
```

with nothing (delete the line).

Update `print_step` (line 110-118) to accept a `total` parameter:

```rust
fn print_step(step: usize, total: usize, description: &str) {
    print!("[Step {:>2}/{}] {} ", step, total, description);
    // Pad dots to align results
    let pad = 40usize.saturating_sub(description.len());
    for _ in 0..pad {
        print!(".");
    }
    print!(" ");
}
```

- [ ] **Step 2: Update `run_step!` macro to use `total_steps` local**

Replace the macro (lines 193-215) with:

```rust
    // A macro to reduce boilerplate for each step
    macro_rules! run_step {
        ($desc:expr, $body:expr) => {{
            step += 1;
            print_step(step, total_steps, $desc);
            match $body.await {
                Ok(detail) => {
                    print_ok(&detail);
                    passed += 1;
                }
                Err(e) => {
                    print_fail(&e);
                    if sender_has_pending_offer {
                        cleanup_sender_offers(&sender, &decentralized_party_id, &registry_url).await;
                    }
                    if receiver_has_pending_offer {
                        println!("Note: receiver may have a pending outgoing offer to cancel manually.");
                    }
                    print_summary(passed, total_steps, start.elapsed().as_secs_f64());
                    return Err(format!("Failed at step {}: {}", step, e));
                }
            }
        }};
    }
```

- [ ] **Step 3: Update consolidation step and final summary**

In the consolidation block (line 377), change:
```rust
        print_step(step, "Consolidate UTXOs (sender)");
```
to:
```rust
        print_step(step, total_steps, "Consolidate UTXOs (sender)");
```

In the consolidation error path (line 401), change:
```rust
                print_summary(passed, TOTAL_STEPS, start.elapsed().as_secs_f64());
```
to:
```rust
                print_summary(passed, total_steps, start.elapsed().as_secs_f64());
```

At the end of main (line 407), change:
```rust
    print_summary(passed, TOTAL_STEPS, start.elapsed().as_secs_f64());
```
to:
```rust
    print_summary(passed, total_steps, start.elapsed().as_secs_f64());
```

- [ ] **Step 4: Add `total_steps` computation and new env vars in `main()`**

After the existing env var loading (after line 178, before the `sender.party_id == receiver.party_id` check), add:

```rust
    let bitsafe_api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");
    let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
        .unwrap_or_else(|_| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
    let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.clone());
    let faucet_url = env::var("FAUCET_URL").ok();
    let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());

    let base_steps: usize = 18;
    let total_steps = base_steps + if faucet_url.is_some() { 3 } else { 0 };
```

After the `sender.party_id == receiver.party_id` check, add early validation:

```rust
    let withdraw_amount_f64: f64 = withdraw_amount
        .parse()
        .expect("WITHDRAW_AMOUNT must be a valid number");
```

- [ ] **Step 5: Add shared mutable variables for new steps**

After `let mut receiver_has_pending_offer = false;` (line 190), add:

```rust
    let mut minter_credential_cids: Vec<String> = Vec::new();
    let mut account_rules: Option<cbtc::mint_redeem::models::AccountContractRuleSet> = None;
    let mut deposit_account: Option<cbtc::mint_redeem::models::DepositAccount> = None;
    let mut withdraw_account: Option<cbtc::mint_redeem::models::WithdrawAccount> = None;
    let mut pre_faucet_count: usize = 0;
    let mut pre_withdraw_balance: f64 = 0.0;
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check --example integration_test`

Expected: compiles with no errors. There will be warnings about unused variables (`withdraw_amount_f64`, `faucet_network`, `pre_faucet_count`, `pre_withdraw_balance`, `deposit_account`, `withdraw_account`, `minter_credential_cids`, `account_rules`) — that's expected and will resolve as we add steps in subsequent tasks.

**Important:** Do NOT run the test at this point. `base_steps` is 18 but only 11 actual steps exist. The output would misleadingly print "FAILED at step 12 of 18" even though all existing steps pass. Only use `cargo check` for verification until all tasks are complete.

- [ ] **Step 7: Commit**

```bash
git add examples/integration_test.rs
git commit -m "refactor: replace TOTAL_STEPS const with dynamic step count

Prepares integration test for new deposit/withdraw/faucet steps.
Step count is now computed at runtime based on FAUCET_URL presence."
```

---

### Task 2: Add credential and account creation steps (steps 3-7)

**Files:**
- Modify: `examples/integration_test.rs` — insert 5 new `run_step!` blocks after step 2 (Check receiver balance)

- [ ] **Step 1: Insert step 3 — Fetch Minter credentials**

After the "Step 2: Check receiver balance" `run_step!` block, add:

```rust
    // Step 3: Fetch Minter credentials (sender)
    run_step!("Fetch Minter credentials", async {
        let token = authenticate(&sender).await?;
        let credentials = cbtc::credentials::list_credentials(cbtc::credentials::ListCredentialsParams {
            ledger_host: sender.ledger_host.clone(),
            party: sender.party_id.clone(),
            access_token: token,
        })
        .await?;

        minter_credential_cids = credentials
            .iter()
            .filter(|c| {
                c.claims
                    .iter()
                    .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
            })
            .map(|c| c.contract_id.clone())
            .collect();

        if minter_credential_cids.is_empty() {
            return Err("No Minter credentials found for sender party".to_string());
        }
        Ok::<String, String>(format!("({} Minter credentials)", minter_credential_cids.len()))
    });
```

- [ ] **Step 2: Insert step 4 — Fetch account rules**

```rust
    // Step 4: Fetch account rules from Bitsafe API
    run_step!("Fetch account rules", async {
        account_rules = Some(
            cbtc::mint_redeem::attestor::get_account_contract_rules(&bitsafe_api_url).await?,
        );
        Ok::<String, String>(format!("(da_rules + wa_rules)"))
    });
```

- [ ] **Step 3: Insert step 5 — Create deposit account**

```rust
    // Step 5: Create deposit account (sender)
    run_step!("Create deposit account", async {
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
            },
        )
        .await?;
        let cid_preview = if account.contract_id.len() > 16 {
            &account.contract_id[..16]
        } else {
            &account.contract_id
        };
        let msg = format!("(owner={}, cid={}...)", account.owner, cid_preview);
        deposit_account = Some(account);
        Ok::<String, String>(msg)
    });
```

- [ ] **Step 4: Insert step 6 — Get Bitcoin address**

```rust
    // Step 6: Get Bitcoin address for deposit account
    run_step!("Get deposit BTC address", async {
        let da = deposit_account.as_ref().unwrap();
        let btc_address = cbtc::mint_redeem::mint::get_bitcoin_address(
            cbtc::mint_redeem::mint::GetBitcoinAddressParams {
                api_url: bitsafe_api_url.clone(),
                account_id: da.account_id().to_string(),
            },
        )
        .await?;
        Ok::<String, String>(format!("({})", btc_address))
    });
```

- [ ] **Step 5: Insert step 7 — Create withdraw account**

```rust
    // Step 7: Create withdraw account (sender)
    run_step!("Create withdraw account", async {
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
            },
        )
        .await?;
        let msg = format!("(dest={})", account.destination_btc_address);
        withdraw_account = Some(account);
        Ok::<String, String>(msg)
    });
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check --example integration_test`

Expected: compiles with no errors (possibly warnings about unused `withdraw_amount_f64`, `faucet_network`, `pre_faucet_count`, `pre_withdraw_balance` — those are used in later tasks).

- [ ] **Step 7: Commit**

```bash
git add examples/integration_test.rs
git commit -m "feat: add credential fetch and account creation steps (3-7)

Steps: fetch Minter credentials, fetch account rules, create deposit
account, get BTC address, create withdraw account."
```

---

### Task 3: Add conditional faucet steps (steps 8-10)

**Files:**
- Modify: `examples/integration_test.rs` — insert conditional `if let` block after step 7

**Faucet API reference:** https://github.com/DLC-link/cbtc-faucet
- Route: `POST {FAUCET_URL}/api/faucet`
- Request body: `{ "network": String, "recipient_party": String, "amount": String }`
- Response body: `{ "success": bool, "message": String, "network": String, "recipient_party": String, "amount": String }`
- Error body: `{ "error": String }` (non-2xx status)
- The faucet submits a `cbtc::transfer::submit()` to the recipient. The recipient must then accept the transfer.

- [ ] **Step 1: Insert faucet block after step 7**

After the "Step 7: Create withdraw account" `run_step!` block, add:

```rust
    // Faucet steps (conditional, only if FAUCET_URL is set)
    // Faucet API: https://github.com/DLC-link/cbtc-faucet
    if let Some(ref faucet_url) = faucet_url {
        // Step 8: Request CBTC from faucet
        run_step!("Request CBTC from faucet", async {
            // Capture baseline incoming count before faucet request
            let token = authenticate(&sender).await?;
            let pre_faucet_incoming = cbtc::utils::fetch_incoming_transfers(
                sender.ledger_host.clone(),
                sender.party_id.clone(),
                token,
            )
            .await?;
            pre_faucet_count = pre_faucet_incoming.len();

            // POST /api/faucet — submits a CBTC transfer to the recipient
            let client = reqwest::Client::new();
            let resp = client
                .post(format!("{}/api/faucet", faucet_url))
                .json(&serde_json::json!({
                    "network": faucet_network,
                    "recipient_party": sender.party_id,
                    "amount": amount,
                }))
                .send()
                .await
                .map_err(|e| format!("Faucet request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Faucet returned status {}: {}", status, body));
            }

            // Verify the response indicates success
            let faucet_resp: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse faucet response: {}", e))?;
            if faucet_resp["success"].as_bool() != Some(true) {
                return Err(format!(
                    "Faucet returned success=false: {}",
                    faucet_resp["message"].as_str().unwrap_or("unknown error")
                ));
            }

            Ok::<String, String>(format!(
                "(requested {} CBTC, {} existing incoming)",
                amount, pre_faucet_count
            ))
        });

        // Step 9: Poll for incoming faucet transfer
        run_step!("Poll for faucet transfer", async {
            let mut attempts = 0;
            let max_attempts = 10;
            let poll_interval = std::time::Duration::from_secs(3);

            loop {
                let token = authenticate(&sender).await?;
                let incoming = cbtc::utils::fetch_incoming_transfers(
                    sender.ledger_host.clone(),
                    sender.party_id.clone(),
                    token,
                )
                .await?;

                if incoming.len() > pre_faucet_count {
                    break;
                }

                attempts += 1;
                if attempts >= max_attempts {
                    return Err("No incoming faucet transfer after 30s".to_string());
                }
                tokio::time::sleep(poll_interval).await;
            }
            Ok::<String, String>(format!("(found after {}s)", attempts * 3))
        });

        // Step 10: Accept faucet transfer
        run_step!("Accept faucet transfer", async {
            let result = cbtc::accept::accept_all(cbtc::accept::AcceptAllParams {
                receiver_party: sender.party_id.clone(),
                ledger_host: sender.ledger_host.clone(),
                registry_url: registry_url.clone(),
                decentralized_party_id: decentralized_party_id.clone(),
                keycloak_client_id: sender.keycloak_client_id.clone(),
                keycloak_username: sender.keycloak_username.clone(),
                keycloak_password: sender.keycloak_password.clone(),
                keycloak_url: sender.keycloak_url.clone(),
            })
            .await?;
            if result.failed_count > 0 {
                return Err(format!("{} accept(s) failed", result.failed_count));
            }
            Ok::<String, String>(format!("({} accepted)", result.successful_count))
        });
    }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --example integration_test`

Expected: compiles. Warnings about unused `withdraw_amount_f64` and `pre_withdraw_balance` remain (used in Task 4).

- [ ] **Step 3: Commit**

```bash
git add examples/integration_test.rs
git commit -m "feat: add conditional faucet deposit steps (8-10)

Request CBTC from faucet, poll for incoming transfer (3s interval,
30s timeout), accept faucet transfer. Only runs if FAUCET_URL is set."
```

---

### Task 4: Add withdrawal and post-withdraw balance steps (steps 15-17)

**Files:**
- Modify: `examples/integration_test.rs` — modify existing step 10 (now step 15), insert 2 new steps before consolidation

- [ ] **Step 1: Modify existing "Check sender balance" (step 15) to capture pre_withdraw_balance**

Find the existing step 10 block (currently the last `run_step!` before consolidation):

```rust
    // Step 10: Check sender balance
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });
```

Replace with:

```rust
    // Step 15: Check sender balance (pre-withdraw)
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender).await?;
        pre_withdraw_balance = balance;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });
```

- [ ] **Step 2: Insert step 16 — Submit withdrawal**

After the modified step 15, before the consolidation block, add:

```rust
    // Step 16: Submit withdrawal (sender)
    run_step!("Submit withdrawal", async {
        let token = authenticate(&sender).await?;
        let wa = withdraw_account.as_ref().unwrap();

        // List holdings and select enough to cover withdraw_amount
        let holdings = cbtc::mint_redeem::redeem::list_holdings(
            cbtc::mint_redeem::redeem::ListHoldingsParams {
                ledger_host: sender.ledger_host.clone(),
                party: sender.party_id.clone(),
                access_token: token.clone(),
            },
        )
        .await?;

        let cbtc_holdings: Vec<_> = holdings
            .iter()
            .filter(|h| h.instrument_id == "CBTC")
            .collect();

        // Greedy select holdings to cover withdraw_amount
        let mut selected = Vec::new();
        let mut selected_total = 0.0;
        for h in &cbtc_holdings {
            let amt = h.amount.parse::<f64>().unwrap_or(0.0);
            selected.push(h.contract_id.clone());
            selected_total += amt;
            if selected_total >= withdraw_amount_f64 {
                break;
            }
        }
        if selected_total < withdraw_amount_f64 {
            return Err(format!(
                "Insufficient holdings: have {}, need {}",
                selected_total, withdraw_amount
            ));
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
            },
        )
        .await?;
        Ok::<String, String>(format!(
            "(burned {} CBTC, pending={})",
            withdraw_amount, updated_account.pending_balance
        ))
    });
```

- [ ] **Step 3: Insert step 17 — Post-withdraw balance check**

```rust
    // Step 17: Check sender balance (post-withdraw)
    run_step!("Check balance (post-withdraw)", async {
        let (balance, utxos) = check_balance(&sender).await?;
        if balance >= pre_withdraw_balance {
            return Err(format!(
                "Balance did not decrease after withdrawal: was {:.8}, now {:.8}",
                pre_withdraw_balance, balance
            ));
        }
        Ok::<String, String>(format!(
            "({:.8} CBTC, {} UTXOs, burned ~{:.8})",
            balance,
            utxos,
            pre_withdraw_balance - balance
        ))
    });
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check --example integration_test`

Expected: compiles with no errors and no warnings about unused variables.

- [ ] **Step 5: Commit**

```bash
git add examples/integration_test.rs
git commit -m "feat: add withdrawal and post-withdraw balance steps (16-17)

Submit withdrawal with limit pre-check, greedy holding selection,
and post-withdraw balance assertion."
```

---

### Task 5: Update doc comment and update step comments

**Files:**
- Modify: `examples/integration_test.rs:1-23` (doc comment), step comments throughout

- [ ] **Step 1: Update the file's doc comment**

Replace lines 1-23 with:

```rust
/// Integration test: end-to-end CBTC flow
///
/// Runs a complete cycle covering:
/// - Credential validation and account setup (deposit + withdraw accounts)
/// - Transfer round-trip: send -> verify -> accept -> return
/// - Withdrawal: burn CBTC and verify balance decrease
/// - UTXO consolidation
/// - (Optional) Faucet deposit if FAUCET_URL is set
///
/// Run with: cargo run --example integration_test
///
/// Required environment variables (sender - standard):
///   KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
///   KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
///   LEDGER_HOST, PARTY_ID
///   DECENTRALIZED_PARTY_ID, REGISTRY_URL
///   BITSAFE_API_URL
///
/// Required environment variables (receiver - RECEIVER_ prefix):
///   RECEIVER_KEYCLOAK_USERNAME, RECEIVER_KEYCLOAK_PASSWORD
///   RECEIVER_KEYCLOAK_CLIENT_ID, RECEIVER_PARTY_ID
///
/// Optional receiver overrides (falls back to sender values):
///   RECEIVER_LEDGER_HOST, RECEIVER_KEYCLOAK_HOST, RECEIVER_KEYCLOAK_REALM
///
/// Optional:
///   TRANSFER_AMOUNT (default: "0.00001")
///   CONSOLIDATION_THRESHOLD (default: "10")
///   DESTINATION_BTC_ADDRESS (default: test address)
///   WITHDRAW_AMOUNT (default: TRANSFER_AMOUNT)
///   FAUCET_URL (if set, enables faucet deposit steps)
///   FAUCET_NETWORK (default: "devnet")
///
/// Note: Deposit and withdraw accounts created during the test are persistent
/// Canton contracts. No cleanup API exists; they remain after the test.
```

- [ ] **Step 2: Update existing step comments to match new numbering**

Update the comment above each existing step to reflect the new step numbers. The existing steps (originally 3-11) are now 8-18 (or 11-21 with faucet). Since `run_step!` auto-increments, the comments are just for human readability. Update them to match the no-faucet flow:

- "Step 3: Send CBTC" → "Step 8: Send CBTC"
- "Step 4: List outgoing" → "Step 9: List outgoing"
- "Step 5: List incoming" → "Step 10: List incoming"
- "Step 6: Accept transfers (receiver)" → "Step 11: Accept transfers (receiver)"
- "Step 7: Check receiver balance" → "Step 12: Check receiver balance"
- "Step 8: Return CBTC" → "Step 13: Return CBTC"
- "Step 9: Accept transfers (sender)" → "Step 14: Accept transfers (sender)"
- "Step 10: Check sender balance" → already updated to "Step 15" in Task 4
- "Step 11: Consolidate" → "Step 18: Consolidate"

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --example integration_test`

Expected: compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add examples/integration_test.rs
git commit -m "docs: update integration test doc comment and step numbering

Add BITSAFE_API_URL, DESTINATION_BTC_ADDRESS, WITHDRAW_AMOUNT,
FAUCET_URL, FAUCET_NETWORK to doc comment. Note about persistent
accounts. Update step comments to match new numbering."
```

---

### Task 6: Final verification

**Files:**
- None modified — verification only

- [ ] **Step 1: Full compile check**

Run: `cargo check --example integration_test`

Expected: compiles with no errors and no warnings.

- [ ] **Step 2: Verify step count logic**

Search the file and confirm:
- `base_steps: usize = 18` — matches the 18-step no-faucet flow
- `if faucet_url.is_some() { 3 }` — matches the 3 faucet steps
- No remaining references to `TOTAL_STEPS`

Run: `grep -n "TOTAL_STEPS" examples/integration_test.rs`

Expected: no matches.

- [ ] **Step 3: Verify no `let` shadowing of shared variables**

Confirm that inside `run_step!` blocks, the shared variables (`minter_credential_cids`, `account_rules`, `deposit_account`, `withdraw_account`, `pre_faucet_count`, `pre_withdraw_balance`) are assigned directly, never with `let` bindings that would shadow them.

Run: `grep -n "let minter_credential_cids\|let account_rules\|let deposit_account\|let withdraw_account\|let pre_faucet_count\|let pre_withdraw_balance" examples/integration_test.rs`

Expected: only the initial declarations in `main()` scope (the `let mut` lines), not inside any async blocks.

- [ ] **Step 4: Count run_step! invocations**

Run: `grep -c "run_step!" examples/integration_test.rs`

Expected: 20 invocations. Breakdown: 10 existing + 7 new (steps 3,4,5,6,7,16,17) + 3 faucet (steps 8,9,10). The consolidation step (18) doesn't use `run_step!`. The `macro_rules! run_step` definition line doesn't contain `!` after the name, so it won't match.

- [ ] **Step 5: Commit if any fixups were needed**

If steps 1-4 revealed issues and you fixed them, commit:

```bash
git add examples/integration_test.rs
git commit -m "fix: address issues found during final verification"
```

If no issues found, skip this step.

---

### Execution Approach: Subagent-Driven

Use `superpowers:subagent-driven-development` to implement this plan. Dispatch a fresh subagent per task, review between tasks.
