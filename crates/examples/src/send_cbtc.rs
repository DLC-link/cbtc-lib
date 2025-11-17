/// Example: Send CBTC to another party
///
/// Run with: cargo run -p examples --example send_cbtc
///
/// Make sure to set up your .env file with the required configuration.
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();

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

    println!("Authenticated successfully!");

    // Set up transfer parameters
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let receiver_party = env::var("LIB_TEST_RECEIVER_PARTY_ID")
        .expect("LIB_TEST_RECEIVER_PARTY_ID must be set (the party to send CBTC to)");
    let amount = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.1".to_string());

    println!("\nSending {} CBTC", amount);
    println!("From: {}", sender_party);
    println!("To: {}", receiver_party);

    // Create transfer
    let decentralized_party =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

    let transfer_params = cbtc::transfer::Params {
        transfer: common::transfer::Transfer {
            sender: sender_party,
            receiver: receiver_party,
            amount,
            instrument_id: common::transfer::InstrumentId {
                admin: decentralized_party.clone(),
                id: "CBTC".to_string(),
            },
            requested_at: chrono::Utc::now().to_rfc3339(),
            execute_before: chrono::Utc::now()
                .checked_add_signed(chrono::Duration::hours(168))
                .unwrap()
                .to_rfc3339(),
            input_holding_cids: None, // Library will auto-select UTXOs
            meta: None,
        },
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        access_token: auth.access_token,
        registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
        decentralized_party_id: decentralized_party,
    };

    // Submit transfer
    println!("\nSubmitting transfer...");
    cbtc::transfer::submit(transfer_params).await?;

    println!("✅ Transfer submitted successfully!");
    println!("\nNote: The receiver must accept the transfer for it to complete.");

    Ok(())
}
