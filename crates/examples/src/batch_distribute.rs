/// Example: Batch distribute CBTC from a CSV file
///
/// Run with: cargo run -p examples --example batch_distribute
///
/// CSV format (recipients.csv):
///   receiver,amount
///   receiver1-party::1220...,5.0
///   receiver2-party::1220...,3.5
///
/// Make sure to set up your .env file with the required configuration.

use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Get CSV file path from environment or use default
    let csv_path = env::var("RECIPIENTS_CSV")
        .unwrap_or_else(|_| "recipients.csv".to_string());

    if !std::path::Path::new(&csv_path).exists() {
        return Err(format!(
            "CSV file not found: {}\n\nCreate a CSV file with format:\nreceiver,amount\nparty1::1220...,5.0\nparty2::1220...,3.5",
            csv_path
        ));
    }

    println!("📦 Batch Distribution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("CSV File: {}", csv_path);

    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let decentralized_party = env::var("DECENTRALIZED_PARTY_ID")
        .expect("DECENTRALIZED_PARTY_ID must be set");

    let batch_params = cbtc::batch::Params {
        csv_path: csv_path.clone(),
        sender: sender_party.clone(),
        instrument_id: common::transfer::InstrumentId {
            admin: decentralized_party.clone(),
            id: "CBTC".to_string(),
        },
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
        decentralized_party_id: decentralized_party,
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
        reference_base: None,
    };

    println!("Sender: {}", sender_party);
    println!("\nProcessing batch distribution...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    cbtc::batch::submit_from_csv(batch_params).await?;

    println!("\n✅ Batch distribution completed successfully!");
    println!("\nNote: Each receiver must accept their transfer for it to complete.");

    Ok(())
}
