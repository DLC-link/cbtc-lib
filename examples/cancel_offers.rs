/// Example: Withdraw Pending Transfers
///
/// This example demonstrates how to withdraw all pending CBTC transfers
/// that you have sent but have not yet been accepted by the receiver.
///
/// Run with: cargo run -p examples --bin cancel_offers
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load configuration from environment
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let registry_url = shared::resolve_registry_url();
    let decentralized_party_id = shared::resolve_party_id();

    let keycloak_client_id =
        env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set");
    let keycloak_username = env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set");
    let keycloak_password = env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set");
    let keycloak_url = keycloak::login::token_url(
        &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
        &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
    );

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Withdraw Pending CBTC Transfers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Sender: {}", sender_party);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Withdraw all pending transfers
    let result = cbtc::cancel_offers::withdraw_all(cbtc::cancel_offers::WithdrawAllParams {
        sender_party,
        instrument_id: cbtc::InstrumentId {
            admin: decentralized_party_id.clone(),
            id: cbtc::CBTC_TICKER.to_string(),
        },
        ledger_host,
        registry_url,
        decentralized_party_id,
        keycloak_client_id,
        keycloak_username,
        keycloak_password,
        keycloak_url,
    })
    .await?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Withdrawal Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Successfully withdrawn: {}", result.successful_count);
    println!("Failed: {}", result.failed_count);

    // Print details of any failures
    let failures: Vec<_> = result.results.iter().filter(|r| !r.success).collect();

    if !failures.is_empty() {
        println!("\nFailed withdrawals:");
        for failure in failures {
            println!(
                "  - Contract {}: {}",
                failure.contract_id,
                failure.error.as_deref().unwrap_or("Unknown error")
            );
        }
    }

    if result.failed_count > 0 {
        return Err(format!(
            "Withdrawal completed with {} failures",
            result.failed_count
        ));
    }

    Ok(())
}
