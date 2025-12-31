/// Example: List Deposit Accounts and Bitcoin Addresses
///
/// This example demonstrates how to:
/// 1. Authenticate with Keycloak
/// 2. List all deposit accounts for your party
/// 3. Fetch the Bitcoin address for each account from the attestor
///
/// Run with: cargo run -p examples --bin list_deposit_addresses
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST, PARTY_ID
/// - ATTESTOR_URL, CANTON_NETWORK
///
/// Note on account IDs:
/// The attestor uses the account's `id` field (a UUID in the createArgument) to
/// look up Bitcoin addresses. For older accounts where this field is null, the
/// `contract_id` is used instead. The `account_id()` method handles this automatically.
use cbtc::mint_redeem::mint::{GetBitcoinAddressParams, ListDepositAccountsParams};
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
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    println!("\nListing deposit accounts for party: {}", party);
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
            account_contract_id: account.account_id().to_string(),
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
