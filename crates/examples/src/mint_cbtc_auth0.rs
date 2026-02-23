/// CBTC Minting Flow Example (Auth0 Version)
///
/// This example demonstrates the complete flow of minting CBTC from BTC using Auth0:
///
/// 1. Authenticate with Auth0 (client_credentials flow)
/// 2. Get account rules from the attestor network
/// 3. Create a deposit account on Canton
/// 4. Get the Bitcoin address for the account
/// 5. (User sends BTC to that address - external step)
/// 6. Monitor for deposit requests
/// 7. Check account status
///
/// To run this example:
/// 1. Make sure .env has AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_CLIENT_SECRET, AUTH0_AUDIENCE
/// 2. cargo run --example mint_cbtc_auth0
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use cbtc::mint_redeem::attestor;
use cbtc::mint_redeem::mint::{
    CreateDepositAccountParams, GetBitcoinAddressParams, GetDepositAccountStatusParams,
    ListDepositAccountsParams,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Minting Flow Example (Auth0) ===\n");

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

    // Step 2: List existing deposit accounts
    println!("Step 2: Listing existing deposit accounts...");
    let accounts = cbtc::mint_redeem::mint::list_deposit_accounts(ListDepositAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    println!("✓ Found {} existing deposit account(s)", accounts.len());
    for account in &accounts {
        println!("  - Contract ID: {}", account.contract_id);
        println!("    Owner: {}", account.owner);
    }
    println!();

    // Step 3: Get account rules from attestor
    println!("Step 3: Getting account contract rules from attestor...");
    let account_rules = attestor::get_account_contract_rules(&attestor_url, &chain).await?;
    println!("✓ Retrieved account rules:");
    println!(
        "  - DepositAccountRules CID: {}",
        account_rules.da_rules.contract_id
    );
    println!(
        "  - WithdrawAccountRules CID: {}",
        account_rules.wa_rules.contract_id
    );
    println!();

    // Step 4: Create a new deposit account
    println!("Step 4: Creating a new deposit account...");
    let deposit_account =
        cbtc::mint_redeem::mint::create_deposit_account(CreateDepositAccountParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            user_name: user_name.clone(),
            access_token: access_token.clone(),
            account_rules: account_rules.clone(),
        })
        .await?;

    println!("✓ Deposit account created successfully!");
    println!("  - Contract ID: {}", deposit_account.contract_id);
    println!("  - Owner: {}", deposit_account.owner);
    println!();

    // Step 5: Get the Bitcoin address for this account
    println!("Step 5: Getting Bitcoin address for the deposit account...");
    let bitcoin_address =
        cbtc::mint_redeem::mint::get_bitcoin_address(GetBitcoinAddressParams {
            attestor_url: attestor_url.clone(),
            account_id: deposit_account.account_id().to_string(),
            chain: chain.clone(),
        })
        .await?;

    println!("✓ Bitcoin address retrieved:");
    println!("  {}", bitcoin_address);
    println!();
    println!("📝 To mint CBTC, send BTC to this address.");
    println!("   Once confirmed, CBTC will be automatically minted to your Canton party.");
    println!();

    // Step 6: Get full account status
    println!("Step 6: Getting full account status...");
    let status =
        cbtc::mint_redeem::mint::get_deposit_account_status(GetDepositAccountStatusParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
            attestor_url: attestor_url.clone(),
            chain: chain.clone(),
            account_contract_id: deposit_account.contract_id.clone(),
        })
        .await?;

    println!("✓ Account status:");
    println!("  - Bitcoin Address: {}", status.bitcoin_address);
    println!("  - Owner: {}", status.owner);
    println!(
        "  - Last Processed BTC Block: {}",
        status.last_processed_bitcoin_block
    );
    println!();

    println!("=== Example Complete ===");
    println!();
    println!("Summary:");
    println!(
        "  • Your deposit account contract ID: {}",
        deposit_account.contract_id
    );
    println!("  • Send BTC to: {}", bitcoin_address);
    println!("  • The attestor network will monitor this address");
    println!("  • Once BTC is confirmed, CBTC will be minted to your party");
    println!();
    println!("To monitor for deposits, you can periodically call:");
    println!("  - get_deposit_account_status() to check account status");

    Ok(())
}

