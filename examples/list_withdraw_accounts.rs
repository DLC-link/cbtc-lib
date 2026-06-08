/// Example: List Withdraw Accounts
///
/// This example demonstrates how to:
/// 1. Authenticate with Keycloak
/// 2. List all withdraw accounts for your party
/// 3. Print each account's destination BTC address and pending balance
///
/// A WithdrawAccount holds the destination Bitcoin address that processed
/// withdrawals are paid out to. Its `pending_balance` reflects CBTC that has
/// been burned but not yet fulfilled on the Bitcoin side by the attestor
/// network.
///
/// Run with: cargo run --example list_withdraw_accounts
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST, PARTY_ID
use cbtc::mint_redeem::redeem::ListWithdrawAccountsParams;
use keycloak::login::{PasswordParams, password_url};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    // Authenticate
    println!("Authenticating...");
    let login_params = PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    let auth = keycloak::login::password(login_params)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");

    println!("\nListing withdraw accounts for party: {}", party);
    println!("{}\n", "=".repeat(60));

    // List all withdraw accounts
    let accounts = cbtc::mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
        ledger_host,
        party,
        access_token: auth.access_token,
    })
    .await?;

    if accounts.is_empty() {
        println!("No withdraw accounts found.");
        return Ok(());
    }

    println!("Found {} withdraw account(s)\n", accounts.len());

    let zero = cbtc::DamlDecimal::ZERO;
    for (i, account) in accounts.iter().enumerate() {
        let status = if account.pending_balance > zero {
            "PENDING"
        } else {
            "ready"
        };
        println!("Account #{}", i + 1);
        println!("  Contract ID:        {}", account.contract_id);
        println!("  Template ID:        {}", account.template_id);
        println!("  Owner:              {}", account.owner);
        println!("  Operator:           {}", account.operator);
        println!("  Registrar:          {}", account.registrar);
        println!("  Destination BTC:    {}", account.destination_btc_address);
        println!("  Pending Balance:    {} BTC [{}]", account.pending_balance, status);
        match &account.limits {
            Some(limits) => {
                let fmt = |v: &Option<cbtc::DamlDecimal>| {
                    v.as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "none".to_string())
                };
                println!(
                    "  Limits:             min={}, max={}",
                    fmt(&limits.min_amount),
                    fmt(&limits.max_amount)
                );
            }
            None => println!("  Limits:             (none)"),
        }
        println!("  Created Event Blob: {}", account.created_event_blob);
        println!();
    }

    Ok(())
}
