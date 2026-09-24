/// Example: read a party's CBTC position through `TokenClient`
///
/// `TokenClient` stores the configuration that otherwise repeats on every
/// call. This example writes nothing: it reads the balance, the UTXO count
/// and the incoming transfer offers.
///
/// It reads one account, not the whole party. A party whose holdings sit
/// under a labelled account sees zero here, while `check_balance` reports the
/// party's full total.
///
/// Run with: cargo run --example token_client
///
/// Requires KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID,
/// KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD, LEDGER_HOST, PARTY_ID and
/// ENVIRONMENT. DECENTRALIZED_PARTY_ID and REGISTRY_URL override it.
use std::env;

use cbtc::{
    CBTC_TICKER, InstrumentId, KeycloakConfig, TokenClient, TokenClientConfig, TokenStandardVersion,
};

mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let required = |name: &str| env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));

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
            // token_url, not the deprecated password_url: no deployed
            // Keycloak serves the legacy {host}/auth/realms/… path.
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
