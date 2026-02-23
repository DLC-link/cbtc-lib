/// Example: Stream CBTC to a Single Receiver (Auth0 Version)
///
/// This script distributes CBTC multiple times to the same receiver.
/// Useful for streaming payments or testing repeated transfers.
///
/// Configuration:
/// - RECEIVER_PARTY: The party ID to receive all transfers
/// - TRANSFER_COUNT: Number of transfers to send
/// - TRANSFER_AMOUNT: Amount per transfer
///
/// Run with: cargo run --example stream_auth0
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use std::env;

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

    println!("Starting stream of {} transfers...\n", transfer_count);

    let mut successful = 0;
    let mut failed = 0;

    for i in 0..transfer_count {
        println!(
            "[{}/{}] Sending {} CBTC to {}...",
            i + 1,
            transfer_count,
            transfer_amount,
            if receiver_party.len() > 30 {
                &receiver_party[..30]
            } else {
                &receiver_party
            }
        );

        let transfer_params = cbtc::transfer::Params {
            transfer: common::transfer::Transfer {
                sender: sender.clone(),
                receiver: receiver_party.clone(),
                amount: transfer_amount.clone(),
                instrument_id: common::transfer::InstrumentId {
                    admin: decentralized_party_id.clone(),
                    id: "CBTC".to_string(),
                },
                requested_at: chrono::Utc::now().to_rfc3339(),
                execute_before: chrono::Utc::now()
                    .checked_add_signed(chrono::Duration::hours(168))
                    .unwrap()
                    .to_rfc3339(),
                input_holding_cids: None,
                meta: None,
            },
            ledger_host: ledger_host.clone(),
            access_token: auth.access_token.clone(),
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
        };

        match cbtc::transfer::submit(transfer_params).await {
            Ok(_) => {
                println!("  ✓ Transfer submitted successfully");
                successful += 1;
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Stream Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);

    if failed > 0 {
        return Err(format!("Stream completed with {} failures", failed));
    }

    Ok(())
}

