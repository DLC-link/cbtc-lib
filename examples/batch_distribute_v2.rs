/// Example: Batch distribute CBTC from a CSV file, Token Standard V2
///
/// Run with: cargo run --example batch_distribute_v2
///
/// CSV format (recipients.csv):
///   receiver,amount
///   party1::1220...,5.0
///
/// The CSV format does not change between versions. Each row's receiver
/// stays a bare party, and the library lifts it to a basic account. What
/// changes is the sender: V2 takes a `cbtc::Account` where V1 takes a
/// party string.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let csv_path = env::var("RECIPIENTS_CSV").unwrap_or_else(|_| "recipients.csv".to_string());
    if !std::path::Path::new(&csv_path).exists() {
        return Err(format!(
            "CSV file not found: {}\n\nCreate a CSV file with format:\nreceiver,amount\nparty1::1220...,5.0",
            csv_path
        ));
    }

    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let decentralized_party_id = shared::resolve_party_id();

    println!("📦 Batch Distribution (V2)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("CSV File: {}", csv_path);
    println!("Sender: {}", sender_party);
    println!("\nProcessing batch distribution...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let result = cbtc::batch::v2::submit_from_csv(cbtc::batch::v2::Params {
        csv_path,
        sender: cbtc::Account::basic(sender_party),
        instrument_id: cbtc::InstrumentId {
            admin: decentralized_party_id.clone(),
            id: cbtc::CBTC_TICKER.to_string(),
        },
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id,
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
        reference_base: None,
    })
    .await?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Successful transfers: {}", result.successful_count);

    if result.failed_count > 0 {
        println!("Failed transfers:     {}", result.failed_count);
        for failed in result.results.iter().filter(|r| !r.success) {
            println!(
                "  {} to {}: {}",
                failed.amount,
                failed.receiver,
                failed.error.as_deref().unwrap_or("no error recorded")
            );
        }
        return Err(format!(
            "{} of {} transfers failed",
            result.failed_count,
            result.results.len()
        ));
    }

    println!("\n✅ Every transfer succeeded.");
    println!("\nNote: Each receiver must accept their transfer for it to complete.");

    Ok(())
}
