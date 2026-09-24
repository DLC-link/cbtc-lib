/// Example: Send CBTC to another party over the Token Standard V2 API
///
/// Run with: cargo run --example send_cbtc_v2
///
/// This is `send_cbtc.rs` on the V2 entry point. The only difference is
/// that V2 addresses accounts rather than bare parties, so `sender` and
/// `receiver` are `Account`s. `Account::basic` builds the account every
/// party owns: no provider, empty id.
///
/// Make sure to set up your .env file with the required configuration.
use std::env;
mod shared;

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
        url: keycloak::login::token_url(
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
    let amount_str = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.1".to_string());
    let amount = cbtc::DamlDecimal::parse(&amount_str).expect("Invalid TRANSFER_AMOUNT");

    println!("\nSending {} CBTC over the V2 API", amount);
    println!("From: {}", sender_party);
    println!("To: {}", receiver_party);

    // Create transfer
    let decentralized_party = shared::resolve_party_id();

    let transfer_params = cbtc::transfer::v2::Params {
        transfer: cbtc::types::v2::Transfer {
            sender: cbtc::Account::basic(sender_party),
            receiver: cbtc::Account::basic(receiver_party),
            amount,
            instrument_id: cbtc::InstrumentId {
                admin: decentralized_party.clone(),
                id: cbtc::CBTC_TICKER.to_string(),
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
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id: decentralized_party,
    };

    // Submit transfer
    println!("\nSubmitting transfer...");
    cbtc::transfer::v2::submit(transfer_params).await?;

    println!("✅ Transfer submitted successfully!");
    println!("\nNote: The receiver must accept the transfer for it to complete.");

    Ok(())
}
