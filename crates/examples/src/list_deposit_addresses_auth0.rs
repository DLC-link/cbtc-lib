/// Example: List Deposit Accounts and Bitcoin Addresses (Auth0 Version)
///
/// This example demonstrates how to:
/// 1. Authenticate with Auth0 (client credentials flow)
/// 2. List all deposit accounts for your party
/// 3. Fetch the Bitcoin address for each account from the attestor
///
/// Run with: cargo run --example list_deposit_addresses_auth0
///
/// Required environment variables:
/// - AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_CLIENT_SECRET, AUTH0_AUDIENCE
/// - LEDGER_HOST, PARTY_ID
/// - ATTESTOR_URL, CANTON_NETWORK
///
/// Note on account IDs:
/// The attestor uses the account's `id` field (a UUID in the createArgument) to
/// look up Bitcoin addresses. For older accounts where this field is null, the
/// `contract_id` is used instead. The `account_id()` method handles this automatically.
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use cbtc::mint_redeem::mint::{GetBitcoinAddressParams, ListDepositAccountsParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    // Authenticate with Auth0
    println!("Authenticating with Auth0...");
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

    let auth = client_credentials(auth_params)
        .await
        .map_err(|e| format!("Auth0 authentication failed: {}", e))?;

    println!("✓ Authenticated successfully!\n");

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    println!("Listing deposit accounts for party: {}", party);
    println!("{}\n", "=".repeat(60));

    // List all deposit accounts
    let accounts = cbtc::mint_redeem::mint::list_deposit_accounts(ListDepositAccountsParams {
        ledger_host,
        party,
        access_token: auth.access_token,
    })
    .await?;

    if accounts.is_empty() {
        println!("No deposit accounts found.");
        return Ok(());
    }

    println!("Found {} deposit account(s)\n", accounts.len());

    // Fetch Bitcoin address for each account
    for (i, account) in accounts.iter().enumerate() {
        println!("Account #{}", i + 1);
        println!("  Contract ID: {}", account.contract_id);
        if let Some(ref id) = account.id {
            println!("  Account ID:  {}", id);
        } else {
            println!("  Account ID:  (none - using contract_id for lookups)");
        }
        println!("  Owner:       {}", account.owner);

        // Fetch the Bitcoin address using account_id() which handles the id/contract_id fallback
        match cbtc::mint_redeem::mint::get_bitcoin_address(GetBitcoinAddressParams {
            attestor_url: attestor_url.clone(),
            account_id: account.account_id().to_string(),
            chain: chain.clone(),
        })
        .await
        {
            Ok(bitcoin_address) => {
                println!("  BTC Address: {}", bitcoin_address);
            }
            Err(e) => {
                println!("  BTC Address: (error: {})", e);
            }
        }
        println!();
    }

    Ok(())
}

