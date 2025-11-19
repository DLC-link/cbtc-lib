/// CBTC Minting Flow Example
///
/// This example demonstrates the complete flow of minting CBTC from BTC:
///
/// 1. Authenticate with Keycloak
/// 2. Get account rules from the attestor network
/// 3. Create a deposit account on Canton
/// 4. Get the Bitcoin address for the account
/// 5. (User sends BTC to that address - simulated with sleep)
/// 6. Monitor for deposit requests
/// 7. Check account status
///
/// To run this example:
/// 1. Copy .env.example to .env and fill in your values
/// 2. cargo run -p examples --bin mint_cbtc_flow
use keycloak::login::{PasswordParams, password, password_url};
use mint_redeem::attestor;
use mint_redeem::mint::{
    CreateDepositAccountParams, GetBitcoinAddressParams, GetDepositAccountStatusParams,
    ListDepositAccountsParams,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Minting Flow Example ===\n");

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

    // Step 2: List existing deposit accounts
    println!("Step 2: Listing existing deposit accounts...");
    let accounts = mint_redeem::mint::list_deposit_accounts(ListDepositAccountsParams {
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
    let deposit_account = mint_redeem::mint::create_deposit_account(CreateDepositAccountParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
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
    let bitcoin_address = mint_redeem::mint::get_bitcoin_address(GetBitcoinAddressParams {
        attestor_url: attestor_url.clone(),
        account_contract_id: deposit_account.contract_id.clone(),
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
    let status = mint_redeem::mint::get_deposit_account_status(GetDepositAccountStatusParams {
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
