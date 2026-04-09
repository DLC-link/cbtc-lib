/// Example: Withdraw Pending Transfers
///
/// This example demonstrates how to withdraw all pending CBTC transfers
/// that you have sent but have not yet been accepted by the receiver.
///
/// Run with: cargo run -p examples --bin cancel_offers
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load configuration from environment
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let registry_url = env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let decentralized_party_id =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Withdraw Pending CBTC Transfers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Sender: {}", sender_party);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Withdraw all pending transfers
    let result = cbtc::cancel_offers::withdraw_all(cbtc::cancel_offers::WithdrawAllParams {
        sender_party,
        ledger_host,
        registry_url,
        decentralized_party_id,
        auth: cbtc::auth::AuthConfig::from_env()?,
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
