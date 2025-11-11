/// Example: Accept all pending CBTC transfers
///
/// Run with: cargo run -p examples --example accept_transfers
///
/// This example uses the `cbtc::accept::accept_all` method to automatically
/// fetch and accept all pending CBTC TransferInstruction contracts for your party.
///
/// Make sure to set up your .env file with the required configuration.
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();

    let params = cbtc::accept::AcceptAllParams {
        receiver_party: env::var("PARTY_ID").expect("PARTY_ID must be set"),
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
        decentralized_party_id: env::var("DECENTRALIZED_PARTY_ID")
            .expect("DECENTRALIZED_PARTY_ID must be set"),
        keycloak_client_id: env::var("KEYCLOAK_CLIENT_ID")
            .expect("KEYCLOAK_CLIENT_ID must be set"),
        keycloak_username: env::var("KEYCLOAK_USERNAME")
            .expect("KEYCLOAK_USERNAME must be set"),
        keycloak_password: env::var("KEYCLOAK_PASSWORD")
            .expect("KEYCLOAK_PASSWORD must be set"),
        keycloak_url: keycloak::login::password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    cbtc::accept::accept_all(params).await?;

    Ok(())
}
