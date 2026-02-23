/// Example: Check CBTC balance and UTXO count (Auth0 Version)
///
/// This example demonstrates how to:
/// 1. Authenticate with Auth0 (client credentials flow)
/// 2. Query active CBTC holdings (UTXOs) for a party
/// 3. Calculate total balance across all UTXOs
/// 4. Monitor UTXO count and warn about consolidation needs
///
/// Run with: cargo run --example check_balance_auth0
///
/// Required environment variables:
/// - AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_CLIENT_SECRET, AUTH0_AUDIENCE
/// - LEDGER_HOST, PARTY_ID
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
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

    println!("\n📊 Checking balance for party: {}", party);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Get active contracts
    let balance_params = cbtc::active_contracts::Params {
        ledger_host,
        party,
        access_token: auth.access_token,
    };

    let holdings = cbtc::active_contracts::get(balance_params).await?;

    // Calculate total balance
    let total_balance: f64 = holdings.iter().filter_map(cbtc::utils::extract_amount).sum();

    // Display results
    println!("Total CBTC Balance: {:.8}", total_balance);
    println!("Number of UTXOs:    {}", holdings.len());
    println!();

    if holdings.len() >= 10 {
        println!("⚠️  Warning: You have {} UTXOs", holdings.len());
        println!("   Canton has a soft limit of 10 UTXOs per party per token.");
        println!("   Consider consolidating your holdings.");
    } else if holdings.len() >= 7 {
        println!("ℹ️  You have {} UTXOs", holdings.len());
        println!("   Consider consolidating soon to stay under the 10 UTXO limit.");
    } else {
        println!("✅ UTXO count is healthy ({}/10)", holdings.len());
    }

    // Show individual holdings
    if !holdings.is_empty() {
        println!("\nIndividual Holdings:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for (i, holding) in holdings.iter().enumerate() {
            let amount = cbtc::utils::extract_amount(holding).unwrap_or(0.0);
            let contract_id = &holding.created_event.contract_id;
            let short_id = if contract_id.len() > 12 {
                format!(
                    "{}...{}",
                    &contract_id[..6],
                    &contract_id[contract_id.len() - 6..]
                )
            } else {
                contract_id.clone()
            };
            println!("  {}. {:.8} CBTC  ({})", i + 1, amount, short_id);
        }
    }

    Ok(())
}

