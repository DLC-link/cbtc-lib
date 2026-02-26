/// Example: List Deposit Accounts using Client Credentials auth
///
/// Same as list_deposit_addresses but authenticates using the client_credentials
/// grant type instead of the password grant.
///
/// Run with: cargo run --example list_deposit_addresses_client_credential
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM
/// - KEYCLOAK_CLIENT_ID, KEYCLOAK_CLIENT_SECRET
/// - LEDGER_HOST, PARTY_ID
/// - ATTESTOR_URL, CANTON_NETWORK
///
/// Optional:
/// - OWNER_FILTER: only show accounts owned by this party
use cbtc::mint_redeem::mint::{GetBitcoinAddressParams, ListDepositAccountsParams};
use keycloak::login::{ClientCredentialsParams, client_credentials_url};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Authenticate using client credentials
    println!("Authenticating...");
    let auth = keycloak::login::client_credentials(ClientCredentialsParams {
        url: client_credentials_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        client_secret: env::var("KEYCLOAK_CLIENT_SECRET")
            .expect("KEYCLOAK_CLIENT_SECRET must be set"),
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");
    let owner_filter = env::var("OWNER_FILTER").ok();

    println!("\nListing deposit accounts for party: {}", party);
    println!("{}\n", "=".repeat(60));

    let all_accounts =
        cbtc::mint_redeem::mint::list_deposit_accounts(ListDepositAccountsParams {
            ledger_host,
            party,
            access_token: auth.access_token,
        })
        .await?;

    let accounts: Vec<_> = match &owner_filter {
        Some(filter) => all_accounts
            .iter()
            .filter(|a| a.owner.contains(filter.as_str()))
            .collect(),
        None => all_accounts.iter().collect(),
    };

    if let Some(ref filter) = owner_filter {
        let full_owner = accounts.first().map(|a| a.owner.as_str()).unwrap_or(filter);
        println!("Filtering by owner: {}", full_owner);
    }

    if accounts.is_empty() {
        println!("No deposit accounts found.");
        return Ok(());
    }

    println!(
        "Found {} deposit account(s){}\n",
        accounts.len(),
        if owner_filter.is_some() {
            format!(" (of {} total)", all_accounts.len())
        } else {
            String::new()
        }
    );

    for (i, account) in accounts.iter().enumerate() {
        println!("Account #{}", i + 1);
        println!("  Contract ID: {}", account.contract_id);
        if let Some(ref id) = account.id {
            println!("  Account ID:  {}", id);
        } else {
            println!("  Account ID:  (none - using contract_id for lookups)");
        }
        println!("  Owner:       {}", account.owner);

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
