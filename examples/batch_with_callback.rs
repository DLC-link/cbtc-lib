use std::future::Future;
/// Example: Batch distribution with callback
///
/// This example demonstrates how to use the callback mechanism to handle
/// transfer results as they complete. The callback can be used for:
/// - Logging transfer results to a file or database
/// - Sending notifications for failed transfers
/// - Tracking progress in real-time
/// - Implementing custom retry logic
use std::pin::Pin;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let csv_path = std::env::var("CSV_PATH").unwrap_or_else(|_| "recipients.csv".to_string());
    let sender = std::env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = std::env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let registry_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let decentralized_party_id =
        std::env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

    let keycloak_client_id =
        std::env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set");
    let keycloak_username =
        std::env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set");
    let keycloak_password =
        std::env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set");
    let keycloak_url = keycloak::login::password_url(
        &std::env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
        &std::env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
    );

    // Read CSV file
    println!("Reading CSV from: {}", csv_path);
    let mut reader =
        csv::Reader::from_path(&csv_path).map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let mut recipients = Vec::new();
    for result in reader.deserialize() {
        let record: CsvRecord = result.map_err(|e| format!("Failed to parse CSV record: {}", e))?;
        recipients.push(cbtc::distribute::Recipient {
            receiver: record.receiver,
            amount: record.amount,
        });
    }

    println!("Found {} recipients", recipients.len());

    // Create log file path
    let log_file = format!(
        "transfer_results_{}.log",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    println!("Logging transfer results to: {}", log_file);

    // Create a callback that writes one line per transfer to a file
    let callback = Box::new(
        move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let log_file = log_file.clone();
            Box::pin(async move {
                use std::fs::OpenOptions;
                use std::io::Write;

                // Build a single line with all relevant info
                let status = if result.success { "SUCCESS" } else { "FAILED" };
                let reference = result.reference.as_deref().unwrap_or("N/A");
                let offer_cid = result.transfer_offer_cid.as_deref().unwrap_or("N/A");
                let update_id = result.update_id.as_deref().unwrap_or("N/A");
                let error = result.error.as_deref().unwrap_or("N/A");

                let log_line = format!(
                    "{} | {} | idx={} | to={} | amount={} | ref={} | offer={} | update_id={} | error={}\n",
                    chrono::Utc::now().to_rfc3339(),
                    status,
                    result.transfer_index,
                    result.receiver,
                    result.amount,
                    reference,
                    offer_cid,
                    update_id,
                    error
                );

                // Write to file
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
                    let _ = file.write_all(log_line.as_bytes());
                }

                // Also print to console
                print!("{}", log_line);
            })
        },
    ) as Box<cbtc::transfer::TransferResultCallback>;

    // Execute batch distribution with callback
    let result = cbtc::distribute::submit(cbtc::distribute::Params {
        recipients,
        sender,
        instrument_id: common::transfer::InstrumentId {
            admin: decentralized_party_id.clone(),
            id: "CBTC".to_string(),
        },
        ledger_host,
        registry_url,
        decentralized_party_id,
        keycloak_client_id,
        keycloak_username,
        keycloak_password,
        keycloak_url,
        reference_base: Some(format!("batch-{}", chrono::Utc::now().timestamp())),
        on_transfer_complete: Some(callback),
    })
    .await?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Batch distribution complete!");
    println!("✓ Successful: {}", result.successful_count);
    println!("✗ Failed: {}", result.failed_count);

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct CsvRecord {
    receiver: String,
    amount: String,
}
