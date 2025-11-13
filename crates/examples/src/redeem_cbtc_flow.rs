/// CBTC Redeeming (Withdrawal) Flow Example
///
/// This example demonstrates the complete flow of redeeming CBTC back to BTC:
///
/// 1. Authenticate with Keycloak
/// 2. Get account rules from the attestor network
/// 3. Create a withdraw account on Canton with destination BTC address
/// 4. List existing CBTC holdings
/// 5. Request withdrawal (burn CBTC and create withdraw request)
/// 6. Monitor withdraw requests
///
/// To run this example:
/// 1. Make sure you have .env configured with your credentials
/// 2. Make sure you have CBTC holdings (run mint_cbtc_flow first)
/// 3. cargo run --example redeem_cbtc_flow

use keycloak::login::{password, password_url, PasswordParams};
use mint_redeem::attestor;
use mint_redeem::redeem::{
    CreateWithdrawAccountParams, ListHoldingsParams, ListWithdrawAccountsParams,
    ListWithdrawRequestsParams, RequestWithdrawParams,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();

    println!("=== CBTC Redeeming (Withdrawal) Flow Example ===\n");

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
    println!("✓ Authenticated successfully\n");

    // Common parameters
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    // Step 2: List existing withdraw accounts
    println!("Step 2: Listing existing withdraw accounts...");
    let accounts =
        mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await?;

    println!("✓ Found {} existing withdraw account(s)", accounts.len());
    for account in &accounts {
        println!("  - Contract ID: {}", account.contract_id);
        println!("    Owner: {}", account.owner);
        println!(
            "    Destination BTC Address: {}",
            account.destination_btc_address
        );
    }
    println!();

    // Step 3: Check CBTC holdings
    println!("Step 3: Checking CBTC holdings...");
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

    println!("✓ Found {} CBTC holding(s)", cbtc_holdings.len());
    println!("  Total CBTC balance: {} BTC", total_cbtc);
    for holding in &cbtc_holdings {
        println!("    - {} BTC (CID: {})", holding.amount, holding.contract_id);
    }
    println!();

    if cbtc_holdings.is_empty() {
        println!("⚠ You don't have any CBTC holdings to redeem.");
        println!("  Run 'mint_cbtc_flow' example first to mint some CBTC.");
        return Ok(());
    }

    // Step 4: Get account rules from attestor
    println!("Step 4: Getting account contract rules from attestor...");
    let account_rules = attestor::get_account_contract_rules(&attestor_url, &chain).await?;
    println!("✓ Retrieved account rules:");
    println!(
        "  - WithdrawAccountRules CID: {}",
        account_rules.wa_rules.contract_id
    );
    println!();

    // Step 5: Create a new withdraw account (or skip if one already exists)
    // For production, you should provide a real Bitcoin address via DESTINATION_BTC_ADDRESS env var
    // For testing/devnet, we use a test address

    if !accounts.is_empty() {
        println!("Step 5: Withdraw account already exists, skipping creation...");
        println!("  Using existing account: {}", accounts[0].contract_id);
        println!("  Destination: {}\n", accounts[0].destination_btc_address);
    } else {
        let destination_btc_address = env::var("DESTINATION_BTC_ADDRESS")
            .unwrap_or_else(|_| "bcrt1qexamplewithdrawaddressfortestingonly00000000".to_string());

        println!("Step 5: Creating a new withdraw account...");
        println!("  Destination BTC address: {}", destination_btc_address);

        let withdraw_account =
            mint_redeem::redeem::create_withdraw_account(CreateWithdrawAccountParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
                access_token: access_token.clone(),
                account_rules_contract_id: account_rules.wa_rules.contract_id.clone(),
                account_rules_template_id: account_rules.wa_rules.template_id.clone(),
                account_rules_created_event_blob: account_rules.wa_rules.created_event_blob.clone(),
                destination_btc_address: destination_btc_address.clone(),
            })
            .await?;

        println!("✓ Withdraw account created successfully!");
        println!("  - Contract ID: {}", withdraw_account.contract_id);
        println!("  - Owner: {}", withdraw_account.owner);
        println!(
            "  - Destination BTC Address: {}",
            withdraw_account.destination_btc_address
        );
        println!();
    }

    // Use the first account (either existing or newly created)
    let withdraw_account = if accounts.is_empty() {
        // Fetch the newly created account
        let updated_accounts = mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await?;
        updated_accounts.into_iter().next().ok_or("Failed to find newly created withdraw account")?
    } else {
        accounts[0].clone()
    };

    // Step 6: Request withdrawal (burn CBTC)
    // For this example, let's try to withdraw a small amount
    let withdraw_amount = "0.001"; // 0.001 BTC
    let withdraw_amount_f64: f64 = withdraw_amount.parse().unwrap();

    if total_cbtc < withdraw_amount_f64 {
        println!(
            "⚠ Insufficient CBTC balance. You have {} but trying to withdraw {}",
            total_cbtc, withdraw_amount
        );
        return Ok(());
    }

    println!("Step 6: Requesting withdrawal (burning CBTC)...");
    println!("  Amount to withdraw: {} BTC", withdraw_amount);

    // Select holdings to burn - for simplicity, just use the first holding with enough balance
    // or combine multiple holdings
    let mut selected_holdings = Vec::new();
    let mut selected_total = 0.0;

    for holding in &cbtc_holdings {
        let amount = holding.amount.parse::<f64>().unwrap_or(0.0);
        selected_holdings.push(holding.contract_id.clone());
        selected_total += amount;

        if selected_total >= withdraw_amount_f64 {
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
            amount: withdraw_amount.to_string(),
            holding_contract_ids: selected_holdings,
        })
        .await?;

    println!("✓ Withdraw request created successfully!");
    println!("  - Contract ID: {}", withdraw_request.contract_id);
    println!("  - Amount: {} BTC", withdraw_request.amount);
    println!(
        "  - Destination: {}",
        withdraw_request.destination_btc_address
    );
    println!("  - BTC TX ID: {}", withdraw_request.btc_tx_id.as_ref().unwrap_or(&"Pending...".to_string()));
    println!();

    // Step 7: List all withdraw requests
    println!("Step 7: Checking all withdraw requests...");
    let withdraw_requests =
        mint_redeem::redeem::list_withdraw_requests(ListWithdrawRequestsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await?;

    if withdraw_requests.is_empty() {
        println!("  No withdraw requests found.");
    } else {
        println!("✓ Found {} withdraw request(s):", withdraw_requests.len());
        for request in &withdraw_requests {
            println!("  - Amount: {} BTC", request.amount);
            println!("    Destination: {}", request.destination_btc_address);
            if let Some(tx_id) = &request.btc_tx_id {
                println!("    BTC TX ID: {} ✓", tx_id);
            } else {
                println!("    Status: Pending attestor processing...");
            }
            println!();
        }
    }

    println!("=== Example Complete ===");
    println!();
    println!("Summary:");
    println!(
        "  • Your withdraw account contract ID: {}",
        withdraw_account.contract_id
    );
    println!("  • Withdraw request created for: {} BTC", withdraw_amount);
    println!(
        "  • BTC will be sent to: {}",
        withdraw_account.destination_btc_address
    );
    println!("  • The attestor network will process your withdrawal request");
    println!("  • Once confirmed, BTC will be sent to your destination address");
    println!();
    println!("To monitor withdrawals, you can periodically call:");
    println!("  - list_withdraw_requests() to see withdrawal status");
    println!("  - Check if btc_tx_id is populated to confirm BTC was sent");

    Ok(())
}
