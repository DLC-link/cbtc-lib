/// Integration test: end-to-end CBTC transfer flow
///
/// Runs a complete send -> verify -> accept -> return -> consolidate cycle
/// between two parties to verify the current environment and network work correctly.
///
/// Run with: cargo run --example integration_test
///
/// Required environment variables (sender - standard):
///   KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
///   KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
///   LEDGER_HOST, PARTY_ID
///   DECENTRALIZED_PARTY_ID, REGISTRY_URL
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
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID")
            .expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::password_url(
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
        keycloak_url: keycloak::login::password_url(&keycloak_host, &keycloak_realm),
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

async fn check_balance(config: &PartyConfig) -> Result<(f64, usize), String> {
    let token = authenticate(config).await?;
    let holdings = cbtc::active_contracts::get(cbtc::active_contracts::Params {
        ledger_host: config.ledger_host.clone(),
        party: config.party_id.clone(),
        access_token: token,
    })
    .await?;

    let total: f64 = holdings
        .iter()
        .filter_map(cbtc::utils::extract_amount)
        .sum();
    Ok((total, holdings.len()))
}

fn print_header(amount: &str) {
    println!();
    println!("===============================================");
    println!("  CBTC Integration Test");
    println!("  Amount: {} CBTC", amount);
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
        println!("  ALL STEPS PASSED ({}/{}) -- {:.1}s", passed, total, elapsed);
    } else {
        println!("  FAILED at step {} of {} -- {:.1}s", passed + 1, total, elapsed);
    }
    println!("===============================================");
    println!();
}

async fn cleanup_sender_offers(sender: &PartyConfig, decentralized_party_id: &str, registry_url: &str) {
    println!("\nAttempting cleanup: canceling pending sender offers...");
    let result = cbtc::cancel_offers::withdraw_all(cbtc::cancel_offers::WithdrawAllParams {
        sender_party: sender.party_id.clone(),
        ledger_host: sender.ledger_host.clone(),
        registry_url: registry_url.to_string(),
        decentralized_party_id: decentralized_party_id.to_string(),
        keycloak_client_id: sender.keycloak_client_id.clone(),
        keycloak_username: sender.keycloak_username.clone(),
        keycloak_password: sender.keycloak_password.clone(),
        keycloak_url: sender.keycloak_url.clone(),
    })
    .await;
    match result {
        Ok(r) => println!("Cleanup: canceled {} offer(s)", r.successful_count),
        Err(e) => println!("Cleanup failed: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let start = Instant::now();
    let sender = load_sender_config();
    let receiver = load_receiver_config();
    let decentralized_party_id =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");
    let registry_url = env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let amount = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.00001".to_string());
    let threshold: usize = env::var("CONSOLIDATION_THRESHOLD")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("CONSOLIDATION_THRESHOLD must be a valid number");

    let bitsafe_api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");
    let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
        .unwrap_or_else(|_| "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx".to_string());
    let withdraw_amount = env::var("WITHDRAW_AMOUNT").unwrap_or_else(|_| amount.clone());
    let faucet_url = env::var("FAUCET_URL").ok();
    let faucet_network = env::var("FAUCET_NETWORK").unwrap_or_else(|_| "devnet".to_string());

    let base_steps: usize = 18;
    let total_steps = base_steps + if faucet_url.is_some() { 3 } else { 0 };

    if sender.party_id == receiver.party_id {
        return Err("Sender and receiver PARTY_ID must be different".to_string());
    }

    let withdraw_amount_f64: f64 = withdraw_amount
        .parse()
        .expect("WITHDRAW_AMOUNT must be a valid number");

    print_header(&amount);

    let mut step = 0;
    let mut passed = 0;
    // Track whether we need cleanup on failure
    let mut sender_has_pending_offer = false;
    let mut receiver_has_pending_offer = false;
    let mut minter_credential_cids: Vec<String> = Vec::new();
    let mut account_rules: Option<cbtc::mint_redeem::models::AccountContractRuleSet> = None;
    let mut deposit_account: Option<cbtc::mint_redeem::models::DepositAccount> = None;
    let mut withdraw_account: Option<cbtc::mint_redeem::models::WithdrawAccount> = None;
    let mut pre_faucet_count: usize = 0;
    let mut pre_withdraw_balance: f64 = 0.0;

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

    // Step 1: Check sender balance
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender).await?;
        if balance <= 0.0 {
            return Err("Sender has no CBTC balance".to_string());
        }
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 2: Check receiver balance
    run_step!("Check receiver balance", async {
        let (balance, utxos) = check_balance(&receiver).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 3: Send CBTC sender -> receiver
    run_step!("Send CBTC to receiver", async {
        let token = authenticate(&sender).await?;
        cbtc::transfer::submit(cbtc::transfer::Params {
            transfer: common::transfer::Transfer {
                sender: sender.party_id.clone(),
                receiver: receiver.party_id.clone(),
                amount: amount.clone(),
                instrument_id: common::transfer::InstrumentId {
                    admin: decentralized_party_id.clone(),
                    id: "CBTC".to_string(),
                },
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
        .await?;
        sender_has_pending_offer = true;
        Ok::<String, String>(format!("({} CBTC)", amount))
    });

    // Step 4: List outgoing offers (sender)
    run_step!("List outgoing offers (sender)", async {
        let token = authenticate(&sender).await?;
        let offers = cbtc::utils::fetch_outgoing_transfers(
            sender.ledger_host.clone(),
            sender.party_id.clone(),
            token,
        )
        .await?;
        if offers.is_empty() {
            return Err("No outgoing offers found after sending".to_string());
        }
        Ok::<String, String>(format!("({} pending)", offers.len()))
    });

    // Step 5: List incoming offers (receiver)
    run_step!("List incoming offers (receiver)", async {
        let token = authenticate(&receiver).await?;
        let offers = cbtc::utils::fetch_incoming_transfers(
            receiver.ledger_host.clone(),
            receiver.party_id.clone(),
            token,
        )
        .await?;
        if offers.is_empty() {
            return Err("No incoming offers found for receiver".to_string());
        }
        Ok::<String, String>(format!("({} pending)", offers.len()))
    });

    // Step 6: Accept transfers (receiver)
    run_step!("Accept transfers (receiver)", async {
        let result = cbtc::accept::accept_all(cbtc::accept::AcceptAllParams {
            receiver_party: receiver.party_id.clone(),
            ledger_host: receiver.ledger_host.clone(),
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
            keycloak_client_id: receiver.keycloak_client_id.clone(),
            keycloak_username: receiver.keycloak_username.clone(),
            keycloak_password: receiver.keycloak_password.clone(),
            keycloak_url: receiver.keycloak_url.clone(),
        })
        .await?;
        sender_has_pending_offer = false;
        if result.failed_count > 0 {
            return Err(format!("{} accept(s) failed", result.failed_count));
        }
        Ok::<String, String>(format!("({} accepted)", result.successful_count))
    });

    // Step 7: Check receiver balance
    run_step!("Check receiver balance", async {
        let (balance, utxos) = check_balance(&receiver).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 8: Return CBTC receiver -> sender
    run_step!("Return CBTC to sender", async {
        let token = authenticate(&receiver).await?;
        cbtc::transfer::submit(cbtc::transfer::Params {
            transfer: common::transfer::Transfer {
                sender: receiver.party_id.clone(),
                receiver: sender.party_id.clone(),
                amount: amount.clone(),
                instrument_id: common::transfer::InstrumentId {
                    admin: decentralized_party_id.clone(),
                    id: "CBTC".to_string(),
                },
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
        .await?;
        receiver_has_pending_offer = true;
        Ok::<String, String>(format!("({} CBTC)", amount))
    });

    // Step 9: Accept transfers (sender)
    run_step!("Accept transfers (sender)", async {
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
        receiver_has_pending_offer = false;
        if result.failed_count > 0 {
            return Err(format!("{} accept(s) failed", result.failed_count));
        }
        Ok::<String, String>(format!("({} accepted)", result.successful_count))
    });

    // Step 10: Check sender balance
    run_step!("Check sender balance", async {
        let (balance, utxos) = check_balance(&sender).await?;
        Ok::<String, String>(format!("({:.8} CBTC, {} UTXOs)", balance, utxos))
    });

    // Step 11: Consolidate UTXOs (sender)
    {
        step += 1;
        print_step(step, total_steps, "Consolidate UTXOs (sender)");
        let token = authenticate(&sender).await.map_err(|e| format!("Auth failed: {}", e))?;
        match cbtc::consolidate::check_and_consolidate(
            cbtc::consolidate::CheckConsolidateParams {
                party: sender.party_id.clone(),
                threshold,
                ledger_host: sender.ledger_host.clone(),
                access_token: token,
                registry_url: registry_url.clone(),
                decentralized_party_id: decentralized_party_id.clone(),
            },
        )
        .await
        {
            Ok(result) => {
                if result.consolidated {
                    print_ok(&format!("({} -> {} UTXOs)", result.utxos_before, result.utxos_after));
                } else {
                    print_skip(&format!("({} < {} threshold)", result.utxos_before, threshold));
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

    print_summary(passed, total_steps, start.elapsed().as_secs_f64());
    Ok(())
}
