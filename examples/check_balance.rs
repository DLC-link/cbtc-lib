/// Example: Check CBTC balance and UTXO count
///
/// This example demonstrates how to:
/// 1. Query active CBTC holdings (UTXOs) for a party
/// 2. Calculate total balance across all UTXOs
/// 3. Monitor UTXO count and warn about consolidation needs
///
/// Run with: cargo run -p examples --bin check_balance
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST, PARTY_ID
///
/// Understanding UTXOs:
/// Each CBTC holding is a separate UTXO (like Bitcoin). Canton has a soft
/// limit of 10 UTXOs per party per token type. Regular consolidation keeps
/// your account healthy and operations efficient.
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    // Authenticate
    println!("Authenticating...");
    let login_params = keycloak::login::PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: keycloak::login::password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    let auth = keycloak::login::password(login_params)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

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
    let total_balance: cbtc::DamlDecimal = holdings
        .iter()
        .filter_map(cbtc::utils::extract_amount)
        .sum();

    // Display results
    println!("Total CBTC Balance: {}", total_balance);
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
            let amount = cbtc::utils::extract_amount(holding)
                .unwrap_or(cbtc::DamlDecimal::parse("0").unwrap());
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
            println!("  {}. {} CBTC  ({})", i + 1, amount, short_id);
        }
    }

    Ok(())
}
