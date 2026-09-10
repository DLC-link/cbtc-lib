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
use std::env;
use std::time::Instant;

struct PartyConfig {
    party_id: String,
    ledger_host: String,
    keycloak_client_id: String,
    keycloak_username: String,
    keycloak_password: String,
    keycloak_url: String,
}

fn load_sender_config() -> PartyConfig {
    PartyConfig {
        party_id: env::var("PARTY_ID").expect("PARTY_ID must be set"),
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    }
}

fn load_receiver_config() -> PartyConfig {
    let keycloak_host = env::var("RECEIVER_KEYCLOAK_HOST")
        .unwrap_or_else(|_| env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"));
    let keycloak_realm = env::var("RECEIVER_KEYCLOAK_REALM")
        .unwrap_or_else(|_| env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"));

    PartyConfig {
        party_id: env::var("RECEIVER_PARTY_ID").expect("RECEIVER_PARTY_ID must be set"),
        ledger_host: env::var("RECEIVER_LEDGER_HOST")
            .unwrap_or_else(|_| env::var("LEDGER_HOST").expect("LEDGER_HOST must be set")),
        keycloak_client_id: env::var("RECEIVER_KEYCLOAK_CLIENT_ID")
            .expect("RECEIVER_KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("RECEIVER_KEYCLOAK_USERNAME")
            .expect("RECEIVER_KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("RECEIVER_KEYCLOAK_PASSWORD")
            .expect("RECEIVER_KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::token_url(&keycloak_host, &keycloak_realm),
    }
}

async fn authenticate(config: &PartyConfig) -> Result<String, String> {
    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: config.keycloak_client_id.clone(),
        username: config.keycloak_username.clone(),
        password: config.keycloak_password.clone(),
        url: config.keycloak_url.clone(),
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;
    Ok(auth.access_token)
}

async fn check_balance(
    config: &PartyConfig,
    instrument: &cbtc::InstrumentId,
    version: cbtc::TokenStandardVersion,
) -> Result<(cbtc::DamlDecimal, usize), String> {
    let token = authenticate(config).await?;
    // A V2 caller reads only its own account's holdings. A V1 caller has no
    // account label to filter on.
    let account = match version {
        cbtc::TokenStandardVersion::V1 => {
            println!("   [v1::active_contracts::get]");
            None
        }
        cbtc::TokenStandardVersion::V2 => {
            println!("   [v2::active_contracts::get]");
            Some(cbtc::Account::basic(config.party_id.clone()))
        }
    };
    let holdings = cbtc::active_contracts::get(cbtc::active_contracts::Params {
        ledger_host: config.ledger_host.clone(),
        party: config.party_id.clone(),
        access_token: token,
        instrument_id: instrument.clone(),
        account,
    })
    .await?;

    let total: cbtc::DamlDecimal = holdings
        .iter()
        .filter_map(cbtc::utils::extract_amount)
        .sum();
    Ok((total, holdings.len()))
}

fn print_header(amount: &str, version: cbtc::TokenStandardVersion) {
    println!();
    println!("===============================================");
    println!("  CBTC Integration Test");
    println!("  Amount: {} CBTC", amount);
    println!("  Token Standard: {:?}", version);
    println!("===============================================");
    println!();
}

fn print_step(step: usize, total: usize, description: &str) {
    print!("[Step {:>2}/{}] {} ", step, total, description);
    let pad = 40usize.saturating_sub(description.len());
    for _ in 0..pad {
        print!(".");
    }
    print!(" ");
}

fn print_ok(detail: &str) {
    println!("OK {}", detail);
}

fn print_fail(detail: &str) {
    println!("FAILED {}", detail);
}

fn print_skip(detail: &str) {
    println!("SKIPPED {}", detail);
}

fn print_summary(passed: usize, total: usize, elapsed: f64) {
    println!();
    println!("===============================================");
    if passed == total {
        println!(
            "  ALL STEPS PASSED ({}/{}) -- {:.1}s",
            passed, total, elapsed
        );
    } else {
        println!(
            "  FAILED at step {} of {} -- {:.1}s",
            passed + 1,
            total,
            elapsed
        );
    }
    println!("===============================================");
    println!();
}

/// The single offer the caller's send just created, by difference against
/// `before`. Returns `None` when the count is not one: another run on the
/// shared wallet sent at the same time, so no single id is safely this run's,
/// and withdrawing somebody else's offer is worse than skipping cleanup.
async fn created_offer(
    sender: &PartyConfig,
    instrument: &cbtc::InstrumentId,
    before: &std::collections::HashSet<String>,
) -> Result<Option<String>, String> {
    let after = outgoing_offer_ids(sender, instrument).await?;
    let created: Vec<&String> = after.difference(before).collect();
    match created.as_slice() {
        [cid] => Ok(Some((*cid).clone())),
        other => {
            println!(
                "   [warning] the send produced {} new offers, not 1",
                other.len()
            );
            Ok(None)
        }
    }
}

/// The ids of the sender's pending outgoing offers for `instrument`.
async fn outgoing_offer_ids(
    sender: &PartyConfig,
    instrument: &cbtc::InstrumentId,
) -> Result<std::collections::HashSet<String>, String> {
    Ok(cbtc::utils::fetch_outgoing_transfers(
        sender.ledger_host.clone(),
        sender.party_id.clone(),
        authenticate(sender).await?,
        instrument.clone(),
    )
    .await?
    .into_iter()
    .map(|c| c.created_event.contract_id)
    .collect())
}

async fn cleanup_sender_offer(
    sender: &PartyConfig,
    decentralized_party_id: &str,
    registry_url: &str,
    contract_id: &str,
    version: cbtc::TokenStandardVersion,
) {
    println!("\nAttempting cleanup: canceling the offer this run created...");
    let token = match authenticate(sender).await {
        Ok(token) => token,
        Err(e) => {
            println!("Cleanup failed to authenticate: {e}");
            return;
        }
    };
    let withdraw_params = cbtc::cancel_offers::Params {
        transfer_offer_contract_id: contract_id.to_string(),
        sender_party: sender.party_id.clone(),
        ledger_host: sender.ledger_host.clone(),
        access_token: token,
        registry_url: registry_url.to_string(),
        decentralized_party_id: decentralized_party_id.to_string(),
    };
    let result = match version {
        cbtc::TokenStandardVersion::V1 => {
            println!("   [v1::cancel_offers::submit]");
            cbtc::cancel_offers::submit(withdraw_params).await
        }
        cbtc::TokenStandardVersion::V2 => {
            println!("   [v2::cancel_offers::submit]");
            cbtc::cancel_offers::v2::submit(withdraw_params).await
        }
    };
    match result {
        Ok(()) => println!("Cleanup: canceled {contract_id}"),
        Err(e) => println!("Cleanup failed: {e}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let version = match env::var("TOKEN_STANDARD_VERSION").as_deref() {
        Ok("V2") | Ok("v2") => cbtc::TokenStandardVersion::V2,
        Ok("V1") | Ok("v1") | Err(_) => cbtc::TokenStandardVersion::V1,
        Ok(other) => {
            return Err(format!(
                "TOKEN_STANDARD_VERSION must be V1 or V2, not {other:?}"
            ));
        }
    };

    let start = Instant::now();
    let sender = load_sender_config();
    let receiver = load_receiver_config();
    let decentralized_party_id =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");
    let instrument = cbtc::InstrumentId {
        admin: decentralized_party_id.clone(),
        id: "CBTC".to_string(),
    };
    let registry_url = env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let amount_str = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.00001".to_string());
    let amount = cbtc::DamlDecimal::parse(&amount_str).expect("Invalid TRANSFER_AMOUNT");
    let threshold: usize = env::var("CONSOLIDATION_THRESHOLD")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("CONSOLIDATION_THRESHOLD must be a valid number");

    let bitsafe_api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");
    let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
    let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.to_string());
    let faucet_url = env::var("FAUCET_URL").ok();
    let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());

    let base_steps: usize = 22;
    let total_steps = base_steps + if faucet_url.is_some() { 3 } else { 0 };

    if sender.party_id == receiver.party_id {
        return Err("Sender and receiver PARTY_ID must be different".to_string());
    }

    let withdraw_amount_decimal = cbtc::DamlDecimal::parse(&withdraw_amount)
        .expect("WITHDRAW_AMOUNT must be a valid number");

    print_header(&amount.to_string(), version);

    let mut step = 0;
    let mut passed = 0;
    // Track whether we need cleanup on failure
    // The offer the sender currently holds, as created by step 9 or step 9c.
    // This doubles as the "has a pending offer" flag: a boolean beside it could
    // disagree with it, and the failure path would then withdraw the wrong
    // offer or none at all. The devnet wallets are shared.
    let mut own_offer_cid: Option<String> = None;
    let mut receiver_has_pending_offer = false;
    let mut minter_credential_cids: Vec<String> = Vec::new();
    let mut account_rules: Option<cbtc::mint_redeem::models::AccountContractRuleSet> = None;
    let mut deposit_account: Option<cbtc::mint_redeem::models::DepositAccount> = None;
    let mut withdraw_account: Option<cbtc::mint_redeem::models::WithdrawAccount> = None;
    let mut pre_faucet_count: usize = 0;
    let mut pre_withdraw_balance = cbtc::DamlDecimal::ZERO;

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
                    match own_offer_cid.clone() {
                        Some(cid) => {
                            cleanup_sender_offer(
                                &sender,
                                &decentralized_party_id,
                                &registry_url,
                                &cid,
                                version,
                            )
                            .await;
                        }
                        None => println!(
                            "Note: this run holds no offer id to cancel. Any pending sender offer needs a manual check."
                        ),
                    }
                    if receiver_has_pending_offer {
                        println!(
                            "Note: receiver may have a pending outgoing offer to cancel manually."
                        );
                    }
                    print_summary(passed, total_steps, start.elapsed().as_secs_f64());
                    return Err(format!("Failed at step {}: {}", step, e));
                }
            }
        }};
    }

    // Step 1: Check sender balance
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender, &instrument, version).await?;
        if balance <= cbtc::DamlDecimal::ZERO {
            return Err("Sender has no CBTC balance".to_string());
        }
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 2: Check receiver balance
    run_step!("Check receiver balance", async {
        let (balance, utxos) = check_balance(&receiver, &instrument, version).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 3: Fetch Minter credentials (sender)
    run_step!("Fetch Minter credentials", async {
        let token = authenticate(&sender).await?;
        let credentials =
            cbtc::credentials::list_credentials(cbtc::credentials::ListCredentialsParams {
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
        Ok::<String, String>(format!(
            "({} Minter credentials)",
            minter_credential_cids.len()
        ))
    });

    // Step 4: Accept free credential offer (gated by RUN_CREDENTIAL_ACCEPT)
    // Exercises credentials.rs:411 parser path. Off by default — accepting a free
    // credential creates a persistent on-ledger contract with no archive choice,
    // so repeated test runs would accumulate state. Set RUN_CREDENTIAL_ACCEPT=1
    // to opt in (same pattern as FAUCET_URL for the optional faucet steps).
    {
        step += 1;
        print_step(step, total_steps, "Accept free credential offer (sender)");
        if env::var("RUN_CREDENTIAL_ACCEPT").ok().as_deref() != Some("1") {
            print_skip("(RUN_CREDENTIAL_ACCEPT not set)");
            passed += 1;
        } else {
            let token = authenticate(&sender)
                .await
                .map_err(|e| format!("Auth failed: {}", e))?;

            let user_service = cbtc::credentials::find_user_service(
                cbtc::credentials::FindUserServiceParams {
                    ledger_host: sender.ledger_host.clone(),
                    party: sender.party_id.clone(),
                    access_token: token.clone(),
                },
            )
            .await
            .map_err(|e| format!("Failed to find UserService: {}", e))?;

            let offers = cbtc::credentials::list_credential_offers(
                cbtc::credentials::ListCredentialOffersParams {
                    ledger_host: sender.ledger_host.clone(),
                    party: sender.party_id.clone(),
                    access_token: token.clone(),
                },
            )
            .await
            .map_err(|e| format!("Failed to list credential offers: {}", e))?;

            match offers.first() {
                None => {
                    print_skip("(no credential offers available)");
                    passed += 1;
                }
                Some(offer) => {
                    match cbtc::credentials::accept_credential_offer(
                        cbtc::credentials::AcceptCredentialOfferParams {
                            ledger_host: sender.ledger_host.clone(),
                            party: sender.party_id.clone(),
                            access_token: token,
                            user_service_template_id: user_service.template_id.clone(),
                            user_service_contract_id: user_service.contract_id.clone(),
                            credential_offer_cid: offer.contract_id.clone(),
                        },
                    )
                    .await
                    {
                        Ok(credential) => {
                            print_ok(&format!("(credential contract_id: {})", credential.contract_id));
                            passed += 1;
                        }
                        Err(e) => {
                            print_fail(&e);
                            print_summary(passed, total_steps, start.elapsed().as_secs_f64());
                            return Err(format!("Failed at step {}: {}", step, e));
                        }
                    }
                }
            }
        }
    }

    // Step 5: Fetch account rules from Bitsafe API
    run_step!("Fetch account rules", async {
        account_rules =
            Some(cbtc::mint_redeem::attestor::get_account_contract_rules(&bitsafe_api_url).await?);
        Ok::<String, String>("(da_rules + wa_rules)".to_string())
    });

    // Step 6: Create deposit account (sender)
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

    // Step 7: Get Bitcoin address for deposit account
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

    // Step 8: Create withdraw account (sender)
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

    // Faucet steps (conditional, only if FAUCET_URL is set)
    // Faucet API: https://github.com/DLC-link/cbtc-faucet
    if let Some(ref faucet_url) = faucet_url {
        // Step 9: Request CBTC from faucet
        run_step!("Request CBTC from faucet", async {
            // Capture baseline incoming count before faucet request
            let token = authenticate(&sender).await?;
            let pre_faucet_incoming = cbtc::utils::fetch_incoming_transfers(
                sender.ledger_host.clone(),
                sender.party_id.clone(),
                token,
                instrument.clone(),
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

        // Step 10: Poll for incoming faucet transfer
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
                    instrument.clone(),
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

        // Step 11: Accept faucet transfer
        run_step!("Accept faucet transfer", async {
            let accept_params = cbtc::accept::AcceptAllParams {
                receiver_party: sender.party_id.clone(),
                instrument_id: instrument.clone(),
                ledger_host: sender.ledger_host.clone(),
                registry_url: registry_url.clone(),
                decentralized_party_id: decentralized_party_id.clone(),
                keycloak_client_id: sender.keycloak_client_id.clone(),
                keycloak_username: sender.keycloak_username.clone(),
                keycloak_password: sender.keycloak_password.clone(),
                keycloak_url: sender.keycloak_url.clone(),
            };
            let result = match version {
                cbtc::TokenStandardVersion::V1 => {
                    println!("   [v1::accept::accept_all]");
                    cbtc::accept::accept_all(accept_params).await?
                }
                cbtc::TokenStandardVersion::V2 => {
                    println!("   [v2::accept::accept_all]");
                    cbtc::accept::v2::accept_all(accept_params).await?
                }
            };
            if result.failed_count > 0 {
                return Err(format!("{} accept(s) failed", result.failed_count));
            }
            Ok::<String, String>(format!("({} accepted)", result.successful_count))
        });
    }

    // Note: Step comments below use no-faucet numbering (9-20).
    // When FAUCET_URL is set, runtime step numbers shift by +3 (becoming 12-23).

    // Step 9: Send CBTC sender -> receiver
    run_step!("Send CBTC to receiver", async {
        let token = authenticate(&sender).await?;
        // Record the sender's offers before the send, so this run can withdraw
        // the one it creates and leave every other offer alone. The devnet
        // wallets are shared.
        let before = outgoing_offer_ids(&sender, &instrument).await?;
        match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::transfer::submit]");
                cbtc::transfer::submit(cbtc::transfer::Params {
                    transfer: cbtc::Transfer {
                        sender: sender.party_id.clone(),
                        receiver: receiver.party_id.clone(),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: sender.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::transfer::submit]");
                cbtc::transfer::v2::submit(cbtc::transfer::v2::Params {
                    transfer: cbtc::types::v2::Transfer {
                        sender: cbtc::Account::basic(sender.party_id.clone()),
                        receiver: cbtc::Account::basic(receiver.party_id.clone()),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: sender.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
        };
        own_offer_cid = created_offer(&sender, &instrument, &before).await?;
        Ok::<String, String>(format!("({} CBTC)", amount))
    });

    // Step 9b: Withdraw the offer step 9 created, and only that one. The
    // devnet wallets are shared, so withdraw_all would cancel offers this run
    // never created. A single withdraw also has no partial-success state.
    run_step!("Withdraw own pending offer", async {
        let cid = own_offer_cid
            .clone()
            .ok_or("step 9 did not identify exactly one new offer to withdraw")?;
        let token = authenticate(&sender).await?;
        let withdraw_params = cbtc::cancel_offers::Params {
            transfer_offer_contract_id: cid.clone(),
            sender_party: sender.party_id.clone(),
            ledger_host: sender.ledger_host.clone(),
            access_token: token,
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
        };
        match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::cancel_offers::submit]");
                cbtc::cancel_offers::submit(withdraw_params).await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::cancel_offers::submit]");
                cbtc::cancel_offers::v2::submit(withdraw_params).await?
            }
        };
        own_offer_cid = None;
        let preview = if cid.len() > 16 { &cid[..16] } else { &cid };
        Ok::<String, String>(format!("({preview})"))
    });

    // Step 9c: Send again, so the steps below have a pending offer to accept.
    run_step!("Send CBTC to receiver", async {
        let token = authenticate(&sender).await?;
        let before = outgoing_offer_ids(&sender, &instrument).await?;
        match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::transfer::submit]");
                cbtc::transfer::submit(cbtc::transfer::Params {
                    transfer: cbtc::Transfer {
                        sender: sender.party_id.clone(),
                        receiver: receiver.party_id.clone(),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: sender.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::transfer::submit]");
                cbtc::transfer::v2::submit(cbtc::transfer::v2::Params {
                    transfer: cbtc::types::v2::Transfer {
                        sender: cbtc::Account::basic(sender.party_id.clone()),
                        receiver: cbtc::Account::basic(receiver.party_id.clone()),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: sender.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
        };
        own_offer_cid = created_offer(&sender, &instrument, &before).await?;
        Ok::<String, String>(format!("({} CBTC)", amount))
    });

    // Step 10: List outgoing offers (sender)
    run_step!("List outgoing offers (sender)", async {
        let token = authenticate(&sender).await?;
        let offers = cbtc::utils::fetch_outgoing_transfers(
            sender.ledger_host.clone(),
            sender.party_id.clone(),
            token,
            instrument.clone(),
        )
        .await?;
        if offers.is_empty() {
            return Err("No outgoing offers found after sending".to_string());
        }
        Ok::<String, String>(format!("({} pending)", offers.len()))
    });

    // Step 11: List incoming offers (receiver)
    run_step!("List incoming offers (receiver)", async {
        let token = authenticate(&receiver).await?;
        let offers = cbtc::utils::fetch_incoming_transfers(
            receiver.ledger_host.clone(),
            receiver.party_id.clone(),
            token,
            instrument.clone(),
        )
        .await?;
        if offers.is_empty() {
            return Err("No incoming offers found for receiver".to_string());
        }
        Ok::<String, String>(format!("({} pending)", offers.len()))
    });

    // Step 12: Accept transfers (receiver)
    run_step!("Accept transfers (receiver)", async {
        let accept_params = cbtc::accept::AcceptAllParams {
            receiver_party: receiver.party_id.clone(),
            instrument_id: instrument.clone(),
            ledger_host: receiver.ledger_host.clone(),
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
            keycloak_client_id: receiver.keycloak_client_id.clone(),
            keycloak_username: receiver.keycloak_username.clone(),
            keycloak_password: receiver.keycloak_password.clone(),
            keycloak_url: receiver.keycloak_url.clone(),
        };
        let result = match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::accept::accept_all]");
                cbtc::accept::accept_all(accept_params).await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::accept::accept_all]");
                cbtc::accept::v2::accept_all(accept_params).await?
            }
        };
        if result.failed_count > 0 {
            // Keep the offer id. A partial accept can leave this run's offer
            // pending, and the failure path below can still cancel it.
            return Err(format!("{} accept(s) failed", result.failed_count));
        }
        own_offer_cid = None;
        Ok::<String, String>(format!("({} accepted)", result.successful_count))
    });

    // Step 13: Check receiver balance
    run_step!("Check receiver balance", async {
        let (balance, utxos) = check_balance(&receiver, &instrument, version).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 14: Return CBTC receiver -> sender
    run_step!("Return CBTC to sender", async {
        let token = authenticate(&receiver).await?;
        match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::transfer::submit]");
                cbtc::transfer::submit(cbtc::transfer::Params {
                    transfer: cbtc::Transfer {
                        sender: receiver.party_id.clone(),
                        receiver: sender.party_id.clone(),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: receiver.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::transfer::submit]");
                cbtc::transfer::v2::submit(cbtc::transfer::v2::Params {
                    transfer: cbtc::types::v2::Transfer {
                        sender: cbtc::Account::basic(receiver.party_id.clone()),
                        receiver: cbtc::Account::basic(sender.party_id.clone()),
                        amount,
                        instrument_id: instrument.clone(),
                        requested_at: chrono::Utc::now().to_rfc3339(),
                        execute_before: chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(168))
                            .unwrap()
                            .to_rfc3339(),
                        input_holding_cids: None,
                        meta: None,
                    },
                    ledger_host: receiver.ledger_host.clone(),
                    access_token: token,
                    registry_url: registry_url.clone(),
                    decentralized_party_id: decentralized_party_id.clone(),
                })
                .await?
            }
        };
        receiver_has_pending_offer = true;
        Ok::<String, String>(format!("({} CBTC)", amount))
    });

    // Step 15: Accept transfers (sender)
    run_step!("Accept transfers (sender)", async {
        let accept_params = cbtc::accept::AcceptAllParams {
            receiver_party: sender.party_id.clone(),
            instrument_id: instrument.clone(),
            ledger_host: sender.ledger_host.clone(),
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
            keycloak_client_id: sender.keycloak_client_id.clone(),
            keycloak_username: sender.keycloak_username.clone(),
            keycloak_password: sender.keycloak_password.clone(),
            keycloak_url: sender.keycloak_url.clone(),
        };
        let result = match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::accept::accept_all]");
                cbtc::accept::accept_all(accept_params).await?
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::accept::accept_all]");
                cbtc::accept::v2::accept_all(accept_params).await?
            }
        };
        if result.failed_count > 0 {
            // Keep the flag, so the failure path still prints the note.
            return Err(format!("{} accept(s) failed", result.failed_count));
        }
        receiver_has_pending_offer = false;
        Ok::<String, String>(format!("({} accepted)", result.successful_count))
    });

    // Step 16: Check sender balance (pre-withdraw)
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender, &instrument, version).await?;
        pre_withdraw_balance = balance;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 17: Submit withdrawal (sender)
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
        let mut selected_total = cbtc::DamlDecimal::ZERO;
        for h in &cbtc_holdings {
            selected.push(h.contract_id.clone());
            selected_total += h.amount;
            if selected_total >= withdraw_amount_decimal {
                break;
            }
        }
        if selected_total < withdraw_amount_decimal {
            return Err(format!(
                "Insufficient holdings: have {}, need {}",
                selected_total, withdraw_amount
            ));
        }

        // Pre-check limits
        cbtc::mint_redeem::models::check_limits("Withdraw", withdraw_amount_decimal, &wa.limits)?;

        // Submit
        let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(
            cbtc::mint_redeem::redeem::SubmitWithdrawParams {
                ledger_host: sender.ledger_host.clone(),
                party: sender.party_id.clone(),
                user_name: sender.keycloak_username.clone(),
                access_token: token,
                api_url: bitsafe_api_url.clone(),
                withdraw_account_contract_id: wa.contract_id.clone(),
                amount: withdraw_amount_decimal,
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

    // Step 18: Check sender balance (post-withdraw)
    run_step!("Check balance (post-withdraw)", async {
        let (balance, utxos) = check_balance(&sender, &instrument, version).await?;
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

    // Step 19: Consolidate UTXOs (sender)
    {
        step += 1;
        print_step(step, total_steps, "Consolidate UTXOs (sender)");
        let token = authenticate(&sender)
            .await
            .map_err(|e| format!("Auth failed: {}", e))?;
        let outcome = match version {
            cbtc::TokenStandardVersion::V1 => {
                println!("   [v1::consolidate::check_and_consolidate]");
                cbtc::consolidate::check_and_consolidate(
                    cbtc::consolidate::CheckConsolidateParams {
                        party: sender.party_id.clone(),
                        instrument_id: instrument.clone(),
                        threshold,
                        ledger_host: sender.ledger_host.clone(),
                        access_token: token,
                        registry_url: registry_url.clone(),
                        decentralized_party_id: decentralized_party_id.clone(),
                    },
                )
                .await
            }
            cbtc::TokenStandardVersion::V2 => {
                println!("   [v2::consolidate::check_and_consolidate]");
                cbtc::consolidate::v2::check_and_consolidate(
                    cbtc::consolidate::v2::CheckConsolidateParams {
                        account: cbtc::Account::basic(sender.party_id.clone()),
                        instrument_id: instrument.clone(),
                        threshold,
                        ledger_host: sender.ledger_host.clone(),
                        access_token: token,
                        registry_url: registry_url.clone(),
                        decentralized_party_id: decentralized_party_id.clone(),
                    },
                )
                .await
            }
        };
        match outcome {
            Ok(result) => {
                if result.consolidated {
                    print_ok(&format!(
                        "({} -> {} UTXOs)",
                        result.utxos_before, result.utxos_after
                    ));
                } else {
                    print_skip(&format!(
                        "({} < {} threshold)",
                        result.utxos_before, threshold
                    ));
                }
                passed += 1;
            }
            Err(e) => {
                print_fail(&e);
                print_summary(passed, total_steps, start.elapsed().as_secs_f64());
                return Err(format!("Failed at step {}: {}", step, e));
            }
        }
    }

    // Step 20: Split sender holding (exercises split.rs parser path end-to-end)
    {
        step += 1;
        print_step(step, total_steps, "Split sender holding");
        let token = authenticate(&sender)
            .await
            .map_err(|e| format!("Auth failed: {}", e))?;

        // List current holdings; pick the first CBTC one to split. Skip if none available.
        let holdings = cbtc::mint_redeem::redeem::list_holdings(
            cbtc::mint_redeem::redeem::ListHoldingsParams {
                ledger_host: sender.ledger_host.clone(),
                party: sender.party_id.clone(),
                access_token: token.clone(),
            },
        )
        .await
        .map_err(|e| format!("Failed to list holdings for split: {}", e))?;

        // Pick a CBTC holding. list_holdings returns every Holding-template contract the
        // sender owns, which on devnet includes legacy `CBTCV0RC8` instruments alongside
        // `CBTC`. Splitting a non-CBTC holding while asserting `instrument_id = "CBTC"`
        // below would make the registry reject the request with 400 "Given holdings are
        // invalid".
        match holdings.iter().find(|h| h.instrument_id == "CBTC") {
            None => {
                print_skip("(no CBTC holdings available to split)");
                passed += 1;
            }
            Some(holding) => {
                // Split the holding into one output worth half its value; the rest becomes change.
                let half = holding.amount / cbtc::DamlDecimal::parse("2").unwrap();

                let outcome = match version {
                    cbtc::TokenStandardVersion::V1 => {
                        println!("   [v1::split::submit]");
                        cbtc::split::submit(cbtc::split::Params {
                            party: sender.party_id.clone(),
                            instrument_id: instrument.clone(),
                            input_holding_cids: vec![holding.contract_id.clone()],
                            amounts: vec![half],
                            ledger_host: sender.ledger_host.clone(),
                            access_token: token,
                            registry_url: registry_url.clone(),
                            decentralized_party_id: decentralized_party_id.clone(),
                        })
                        .await
                    }
                    cbtc::TokenStandardVersion::V2 => {
                        println!("   [v2::split::submit]");
                        cbtc::split::v2::submit(cbtc::split::v2::Params {
                            account: cbtc::Account::basic(sender.party_id.clone()),
                            instrument_id: instrument.clone(),
                            input_holding_cids: vec![holding.contract_id.clone()],
                            amounts: vec![half],
                            ledger_host: sender.ledger_host.clone(),
                            access_token: token,
                            registry_url: registry_url.clone(),
                            decentralized_party_id: decentralized_party_id.clone(),
                        })
                        .await
                    }
                };

                match outcome {
                    Ok(result) => {
                        print_ok(&format!(
                            "({} output(s), {} change UTXO(s))",
                            result.output_holding_cids.len(),
                            result.change_holding_cids.len()
                        ));
                        passed += 1;
                    }
                    Err(e) => {
                        print_fail(&format!(
                            "{} (partial: {} output(s), {} change UTXO(s))",
                            e.message,
                            e.partial.output_holding_cids.len(),
                            e.partial.change_holding_cids.len()
                        ));
                        print_summary(passed, total_steps, start.elapsed().as_secs_f64());
                        return Err(format!("Failed at step {}: {}", step, e.message));
                    }
                }
            }
        }
    }

    print_summary(passed, total_steps, start.elapsed().as_secs_f64());
    Ok(())
}
