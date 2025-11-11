/// Test burning CBTC using an existing withdraw account
///
/// This example demonstrates burning a small amount of CBTC using an existing
/// withdraw account instead of creating a new one.
///
/// Usage:
/// cargo run --example test_burn_cbtc

use keycloak::login::{password, password_url, PasswordParams};
use mint_redeem::redeem::{
    ListHoldingsParams, ListWithdrawAccountsParams, ListWithdrawRequestsParams,
    RequestWithdrawParams,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();

    println!("=== Test CBTC Burn (Withdraw) ===\n");

    // Authenticate
    println!("Authenticating...");
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
    println!("✓ Authenticated\n");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    // List existing withdraw accounts
    println!("Listing existing withdraw accounts...");
    let accounts =
        mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await?;

    if accounts.is_empty() {
        println!("❌ No withdraw accounts found.");
        println!("   Please run 'redeem_cbtc_flow' example first to create a withdraw account.");
        return Ok(());
    }

    println!("✓ Found {} withdraw account(s)", accounts.len());
    let withdraw_account = &accounts[0];
    println!("  Using withdraw account: {}", withdraw_account.contract_id);
    println!(
        "  Destination BTC address: {}\n",
        withdraw_account.destination_btc_address
    );

    // Check CBTC holdings
    println!("Checking CBTC holdings...");
    let holdings = mint_redeem::redeem::list_holdings(ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let cbtc_holdings: Vec<_> = holdings
        .iter()
        .filter(|h| h.instrument_id == "CBTC")
        .collect();

    let total_cbtc: f64 = cbtc_holdings
        .iter()
        .map(|h| h.amount.parse::<f64>().unwrap_or(0.0))
        .sum();

    println!("✓ Total CBTC balance: {} BTC", total_cbtc);
    println!("  Found {} holding(s)\n", cbtc_holdings.len());

    if cbtc_holdings.is_empty() {
        println!("❌ You don't have any CBTC holdings to burn.");
        return Ok(());
    }

    // Burn a small amount
    let burn_amount = "0.0001"; // 0.0001 BTC
    let burn_amount_f64: f64 = burn_amount.parse().unwrap();

    if total_cbtc < burn_amount_f64 {
        println!(
            "⚠ Insufficient CBTC balance. You have {} but trying to burn {}",
            total_cbtc, burn_amount
        );
        return Ok(());
    }

    println!("Burning {} BTC...", burn_amount);

    // Select holdings to burn
    let mut selected_holdings = Vec::new();
    let mut selected_total = 0.0;

    for holding in &cbtc_holdings {
        let amount = holding.amount.parse::<f64>().unwrap_or(0.0);
        selected_holdings.push(holding.contract_id.clone());
        selected_total += amount;

        if selected_total >= burn_amount_f64 {
            break;
        }
    }

    println!("  Using {} holding(s) totaling {} BTC", selected_holdings.len(), selected_total);

    let withdraw_request =
        mint_redeem::redeem::request_withdraw(RequestWithdrawParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            access_token: access_token.clone(),
            attestor_url: attestor_url.clone(),
            chain: chain.clone(),
            withdraw_account_contract_id: withdraw_account.contract_id.clone(),
            amount: burn_amount.to_string(),
            holding_contract_ids: selected_holdings,
        })
        .await?;

    println!("\n✓ Withdraw request created successfully!");
    println!("  - Contract ID: {}", withdraw_request.contract_id);
    println!("  - Amount: {} BTC", withdraw_request.amount);
    println!(
        "  - Destination: {}",
        withdraw_request.destination_btc_address
    );

    if let Some(tx_id) = &withdraw_request.btc_tx_id {
        println!("  - BTC TX ID: {} ✓", tx_id);
    } else {
        println!("  - Status: Pending attestor processing...");
    }

    // List all withdraw requests
    println!("\nChecking all withdraw requests...");
    let withdraw_requests =
        mint_redeem::redeem::list_withdraw_requests(ListWithdrawRequestsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await?;

    println!("✓ Total withdraw requests: {}", withdraw_requests.len());
    for (i, request) in withdraw_requests.iter().enumerate().take(5) {
        println!("  {}. {} BTC to {}", i + 1, request.amount, request.destination_btc_address);
        if let Some(tx_id) = &request.btc_tx_id {
            println!("     BTC TX: {} ✓", tx_id);
        } else {
            println!("     Status: Pending...");
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
