/// Example: Distribute CBTC with a per-transfer callback, Token Standard V2
///
/// Run with: cargo run --example batch_with_callback_v2
///
/// CSV format (recipients.csv):
///   receiver,amount
///   party1::1220...,5.0
///
/// The callback fires once per transfer, so a long run reports as it goes
/// rather than at the end. `distribute::v2` returns the same result as V1,
/// with every transfer's outcome and the two counts.
///
/// V2 takes a `cbtc::Account` for the sender and for each recipient, where
/// V1 takes party strings.
use std::env;
use std::fs::OpenOptions;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;
mod shared;

#[derive(Debug, serde::Deserialize)]
struct CsvRecord {
    receiver: String,
    amount: String,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let csv_path = env::var("RECIPIENTS_CSV").unwrap_or_else(|_| "recipients.csv".to_string());
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let decentralized_party_id = shared::resolve_party_id();

    println!("Reading CSV from: {}", csv_path);
    let mut reader =
        csv::Reader::from_path(&csv_path).map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let mut recipients = Vec::new();
    for record in reader.deserialize() {
        let record: CsvRecord = record.map_err(|e| format!("Failed to parse CSV record: {}", e))?;
        let amount = cbtc::DamlDecimal::parse(&record.amount)
            .map_err(|e| format!("Invalid amount '{}': {}", record.amount, e))?;
        recipients.push(cbtc::distribute::v2::Recipient {
            receiver: cbtc::Account::basic(record.receiver),
            amount,
        });
    }

    println!("Found {} recipients", recipients.len());

    let log_file = format!(
        "transfer_results_{}.log",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    println!("Logging transfer results to: {}", log_file);

    let callback = Box::new(
        move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let log_file = log_file.clone();
            Box::pin(async move {
                let line = format!(
                    "[{}] {} {} to {}{}\n",
                    chrono::Utc::now().to_rfc3339(),
                    if result.success { "OK  " } else { "FAIL" },
                    result.amount,
                    result.receiver,
                    result
                        .error
                        .as_deref()
                        .map(|e| format!(": {}", e))
                        .unwrap_or_default()
                );

                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
                    let _ = file.write_all(line.as_bytes());
                }
                print!("{}", line);
            })
        },
    ) as Box<cbtc::transfer::TransferResultCallback>;

    let result = cbtc::distribute::v2::submit(cbtc::distribute::v2::Params {
        recipients,
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
        reference_base: Some(format!("batch-v2-{}", chrono::Utc::now().timestamp())),
        on_transfer_complete: Some(callback),
    })
    .await?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Batch distribution complete!");
    println!("✓ Successful: {}", result.successful_count);
    println!("✗ Failed: {}", result.failed_count);

    if result.failed_count > 0 {
        return Err(format!(
            "{} of {} transfers failed",
            result.failed_count,
            result.results.len()
        ));
    }

    Ok(())
}
