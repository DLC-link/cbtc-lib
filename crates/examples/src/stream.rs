/// Example: Stream CBTC to a Single Receiver
///
/// This script distributes CBTC multiple times to the same receiver.
/// Useful for streaming payments or testing repeated transfers.
///
/// Configuration:
/// - RECEIVER_PARTY: The party ID to receive all transfers
/// - TRANSFER_COUNT: Number of transfers to send
/// - TRANSFER_AMOUNT: Amount per transfer
///
/// Run with: cargo run -p examples --bin stream
use std::env;
use std::future::Future;
use std::pin::Pin;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load configuration
    let sender = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let receiver_party = env::var("RECEIVER_PARTY").expect("RECEIVER_PARTY must be set");
    let transfer_count: usize = env::var("TRANSFER_COUNT")
        .expect("TRANSFER_COUNT must be set")
        .parse()
        .expect("TRANSFER_COUNT must be a valid number");
    let transfer_amount = env::var("TRANSFER_AMOUNT").expect("TRANSFER_AMOUNT must be set");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let registry_url = env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let decentralized_party_id =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

    let keycloak_client_id =
        env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set");
    let keycloak_username = env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set");
    let keycloak_password = env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set");
    let keycloak_url = keycloak::login::password_url(
        &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
        &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
    );

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Stream CBTC Configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Sender: {}", sender);
    println!("Receiver: {}", receiver_party);
    println!("Transfer count: {}", transfer_count);
    println!("Amount per transfer: {}", transfer_amount);
    println!(
        "Total amount: {} CBTC",
        transfer_count as f64 * transfer_amount.parse::<f64>().unwrap_or(0.0)
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Create log file
    let log_file = format!(
        "stream_results_{}.log",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    println!("Logging results to: {}", log_file);

    let callback = Box::new(
        move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
            let log_file = log_file.clone();
            Box::pin(async move {
                use std::fs::OpenOptions;
                use std::io::Write;

                let status = if result.success { "SUCCESS" } else { "FAILED" };
                let reference = result.reference.as_deref().unwrap_or("N/A");
                let offer_cid = result.transfer_offer_cid.as_deref().unwrap_or("N/A");
                let update_id = result.update_id.as_deref().unwrap_or("N/A");
                let error = result.error.as_deref().unwrap_or("N/A");

                let log_line = format!(
                    "{} | {} | idx={} | to={} | amount={} | ref={} | offer={} | update_id={} | error={} | raw={}\n",
                    chrono::Utc::now().to_rfc3339(),
                    status,
                    result.transfer_index,
                    result.receiver,
                    result.amount,
                    reference,
                    offer_cid,
                    update_id,
                    error,
                    result.raw_response.as_deref().unwrap_or("N/A")
                );

                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
                    let _ = file.write_all(log_line.as_bytes());
                }

                print!("{}", log_line);
            })
        },
    ) as Box<cbtc::transfer::TransferResultCallback>;

    // Create recipients: same receiver, repeated transfer_count times
    let recipients: Vec<cbtc::distribute::Recipient> = (0..transfer_count)
        .map(|_| cbtc::distribute::Recipient {
            receiver: receiver_party.clone(),
            amount: transfer_amount.clone(),
        })
        .collect();

    println!("\nStarting stream of {} transfers...\n", transfer_count);

    let result = cbtc::distribute::submit(cbtc::distribute::Params {
        recipients,
        sender: sender.clone(),
        instrument_id: common::transfer::InstrumentId {
            admin: decentralized_party_id.clone(),
            id: "CBTC".to_string(),
        },
        ledger_host: ledger_host.clone(),
        registry_url: registry_url.clone(),
        decentralized_party_id: decentralized_party_id.clone(),
        keycloak_client_id: keycloak_client_id.clone(),
        keycloak_username: keycloak_username.clone(),
        keycloak_password: keycloak_password.clone(),
        keycloak_url: keycloak_url.clone(),
        reference_base: Some(format!("stream-{}", chrono::Utc::now().timestamp())),
        on_transfer_complete: Some(callback),
    })
    .await?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Stream Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Successful: {}", result.successful_count);
    println!("Failed: {}", result.failed_count);

    if result.failed_count > 0 {
        return Err(format!(
            "Stream completed with {} failures",
            result.failed_count
        ));
    }

    Ok(())
}
