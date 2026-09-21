/// Example: read a party's CBTC position through `TokenClient`
///
/// `TokenClient` stores the configuration that otherwise repeats on
/// every call, and it authenticates itself, so this example makes no
/// Keycloak call of its own.
///
/// This example writes nothing. It reads the balance, the UTXO count and
/// the incoming transfer offers.
///
/// Run with: cargo run --example token_client
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST, PARTY_ID
/// - ENVIRONMENT (devnet, testnet or mainnet)
///
/// Optional overrides, for a network ENVIRONMENT cannot name:
/// DECENTRALIZED_PARTY_ID, REGISTRY_URL
use std::env;

use cbtc::{
    CBTC_TICKER, InstrumentId, KeycloakConfig, TokenClient, TokenClientConfig,
    TokenStandardVersion,
};

mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let required =
        |name: &str| env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));

    let mut client = TokenClient::connect(TokenClientConfig {
        ledger_host: required("LEDGER_HOST"),
        registry_url: shared::resolve_registry_url(),
        instrument: InstrumentId {
            admin: shared::resolve_party_id(),
            id: CBTC_TICKER.to_string(),
        },
        party: required("PARTY_ID"),
        keycloak: KeycloakConfig {
            client_id: required("KEYCLOAK_CLIENT_ID"),
            username: required("KEYCLOAK_USERNAME"),
            password: required("KEYCLOAK_PASSWORD"),
            // token_url, not password_url. This field's own doc comment
            // recommends password_url, which canton-lib deprecated in
            // 0.5.1: it emits the legacy {host}/auth/realms/… path, and
            // no deployed Keycloak serves that prefix.
            url: keycloak::login::token_url(
                &required("KEYCLOAK_HOST"),
                &required("KEYCLOAK_REALM"),
            ),
        },
        version: TokenStandardVersion::V2,
    })
    .await?;

    println!("party      {}", client.party());
    println!(
        "instrument {}:{}",
        client.instrument().admin,
        client.instrument().id
    );
    println!("balance    {}", client.balance().await?);
    println!("utxos      {}", client.utxo_count().await?);
    println!(
        "incoming   {} offer(s)",
        client.incoming_offers().await?.len()
    );

    Ok(())
}
