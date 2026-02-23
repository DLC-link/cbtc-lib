/// CBTC Redeeming (Withdrawal) Flow Example (Auth0 Version)
///
/// This example demonstrates the complete flow of submitting a CBTC withdrawal using Auth0:
///
/// 1. Authenticate with Auth0 (client_credentials flow)
/// 2. Get account rules from the attestor network
/// 3. Create a withdraw account on Canton with destination BTC address
/// 4. List existing CBTC holdings
/// 5. Submit withdrawal (burn CBTC and increase pending balance)
/// 6. Verify the withdrawal was submitted successfully
///
/// Note: WithdrawRequests are NOT created atomically with the withdrawal submission.
/// The attestor network will create WithdrawRequests later. Use the separate
/// `check_withdraw_requests` example to monitor for processed withdrawals.
///
/// To run this example:
/// 1. Make sure .env has AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_CLIENT_SECRET, AUTH0_AUDIENCE
/// 2. Make sure you have CBTC holdings (run mint_cbtc_auth0 first)
/// 3. cargo run --example redeem_cbtc_auth0
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use cbtc::mint_redeem::attestor;
use cbtc::mint_redeem::redeem::{
    CreateWithdrawAccountParams, ListHoldingsParams, ListWithdrawAccountsParams,
    SubmitWithdrawParams,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Redeeming (Withdrawal) Flow Example (Auth0) ===\n");

    // Step 1: Authenticate with Auth0
    println!("Step 1: Authenticating with Auth0...");
    let auth0_domain = env::var("AUTH0_DOMAIN").map_err(|_| "AUTH0_DOMAIN must be set")?;
    let auth0_client_id = env::var("AUTH0_CLIENT_ID").map_err(|_| "AUTH0_CLIENT_ID must be set")?;
    let auth0_client_secret =
        env::var("AUTH0_CLIENT_SECRET").map_err(|_| "AUTH0_CLIENT_SECRET must be set")?;
    let auth0_audience = env::var("AUTH0_AUDIENCE").map_err(|_| "AUTH0_AUDIENCE must be set")?;

    let auth_params = ClientCredentialsParams {
        url: auth0_url(&auth0_domain),
        client_id: auth0_client_id,
        client_secret: auth0_client_secret,
        audience: auth0_audience,
    };

    let login_response = client_credentials(auth_params)
        .await
        .map_err(|e| format!("Auth0 authentication failed: {}", e))?;

    println!("✓ Authenticated successfully!");
    println!("  Token expires in: {} seconds", login_response.expires_in);

    // Extract user identifier from token for account creation
    let user_name = login_response
        .get_user_id()
        .unwrap_or_else(|_| "auth0-user".to_string());
    println!("  User ID: {}\n", user_name);

    // Common parameters
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    // Step 2: List existing withdraw accounts
    println!("Step 2: Listing existing withdraw accounts...");
    let accounts =
        cbtc::mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
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
    let holdings = cbtc::mint_redeem::redeem::list_holdings(ListHoldingsParams {
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
        println!(
            "    - {} BTC (CID: {})",
            holding.amount, holding.contract_id
        );
    }
    println!();

    if cbtc_holdings.is_empty() {
        println!("⚠ You don't have any CBTC holdings to redeem.");
        println!("  Run 'cargo run --example mint_cbtc_auth0' first to mint some CBTC.");
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
            cbtc::mint_redeem::redeem::create_withdraw_account(CreateWithdrawAccountParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                user_name: user_name.clone(),
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
        let updated_accounts =
            cbtc::mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                access_token: access_token.clone(),
            })
            .await?;
        updated_accounts
            .into_iter()
            .next()
            .ok_or("Failed to find newly created withdraw account")?
    } else {
        accounts[0].clone()
    };

    // Step 6: Submit withdrawal (burn CBTC)
    let withdraw_amount = "0.00003218";
    let withdraw_amount_f64: f64 = withdraw_amount.parse().unwrap();

    if total_cbtc < withdraw_amount_f64 {
        println!(
            "⚠ Insufficient CBTC balance. You have {} but trying to withdraw {}",
            total_cbtc, withdraw_amount
        );
        return Ok(());
    }

    println!("Step 6: Submitting withdrawal (burning CBTC)...");
    println!("  Amount to withdraw: {} BTC", withdraw_amount);

    // Select holdings to burn
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

    println!(
        "  Using {} holding(s) totaling {} BTC",
        selected_holdings.len(),
        selected_total
    );

    let updated_account = cbtc::mint_redeem::redeem::submit_withdraw(SubmitWithdrawParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        user_name: user_name.clone(),
        access_token: access_token.clone(),
        attestor_url: attestor_url.clone(),
        chain: chain.clone(),
        withdraw_account_contract_id: withdraw_account.contract_id.clone(),
        withdraw_account_template_id: withdraw_account.template_id.clone(),
        withdraw_account_created_event_blob: withdraw_account.created_event_blob.clone(),
        amount: withdraw_amount.to_string(),
        holding_contract_ids: selected_holdings,
    })
    .await?;

    println!("✓ Withdrawal submitted successfully!");
    println!(
        "  - Updated Account Contract ID: {}",
        updated_account.contract_id
    );
    println!("  - Pending Balance: {} BTC", updated_account.pending_balance);
    println!(
        "  - Destination: {}",
        updated_account.destination_btc_address
    );
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Summary:");
    println!(
        "  • Your withdraw account contract ID: {}",
        updated_account.contract_id
    );
    println!("  • Pending balance: {} BTC", updated_account.pending_balance);
    println!(
        "  • BTC will be sent to: {}",
        updated_account.destination_btc_address
    );
    println!();
    println!("Important: WithdrawRequests are NOT created atomically with this call.");
    println!("The attestor network will process your pending balance and create a");
    println!("WithdrawRequest later. Use 'check_withdraw_requests' to monitor:");
    println!("  cargo run --example check_withdraw_requests_auth0");

    Ok(())
}

