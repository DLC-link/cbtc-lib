//! Read-only smoke test of the Ledger API request formats canton-lib sends.
//!
//! Runs the HTTP and websocket active-contracts queries and holds an update
//! subscription open for 15 seconds. Nothing is submitted to the ledger.
//!
//! Run with: cargo run --example ledger_api_smoke

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ledger::common;

static UPDATE_MESSAGES: AtomicUsize = AtomicUsize::new(0);

const HOLDING_INTERFACE: &str = "#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding";

fn holding_filter() -> common::IdentifierFilter {
    common::IdentifierFilter::InterfaceIdentifierFilter(common::InterfaceIdentifierFilter {
        interface_filter: common::InterfaceFilter {
            value: common::InterfaceFilterValue {
                interface_id: Some(HOLDING_INTERFACE.to_string()),
                include_interface_view: true,
                include_created_event_blob: false,
            },
        },
    })
}

fn on_update(message: String) -> Result<(), String> {
    let n = UPDATE_MESSAGES.fetch_add(1, Ordering::SeqCst);
    if n == 0 {
        let preview: String = message.chars().take(200).collect();
        println!("   first update message: {preview}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");

    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: keycloak::login::token_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    })
    .await
    .map_err(|e| format!("Authentication failed: {e}"))?;
    let token = auth.access_token;

    let version = reqwest::Client::new()
        .get(format!("{}/v2/version", ledger_host.trim_end_matches('/')))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("GET /v2/version failed: {e}"))?
        .text()
        .await
        .map_err(|e| format!("GET /v2/version body: {e}"))?;
    let version: serde_json::Value =
        serde_json::from_str(&version).map_err(|e| format!("version JSON: {e}"))?;
    println!("participant version: {}", version["version"]);

    let ledger_end = ledger::ledger_end::get(ledger::ledger_end::Params {
        access_token: token.clone(),
        ledger_host: ledger_host.clone(),
    })
    .await?
    .offset;
    println!("ledger end: {ledger_end}");

    let http = ledger::active_contracts::get_by_party(ledger::active_contracts::Params {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        filter: holding_filter(),
        access_token: token.clone(),
        ledger_end,
        unknown_contract_entry_handler: None,
    })
    .await?;
    println!("HTTP active contracts (Holding): {}", http.len());

    let ws =
        ledger::websocket::active_contracts::get(ledger::websocket::active_contracts::Params {
            ledger_host: ledger_host.clone(),
            party: party.clone(),
            filter: holding_filter(),
            access_token: token.clone(),
            ledger_end,
        })
        .await?;
    println!("websocket active contracts (Holding): {}", ws.len());

    if http.len() != ws.len() {
        return Err(format!(
            "HTTP and websocket counts differ: {} vs {}",
            http.len(),
            ws.len()
        ));
    }

    // Subscribe from an offset behind the ledger end so that past updates for
    // the party, if any, arrive straight away.
    let begin = (ledger_end - 1_000).max(0);
    let subscription = tokio::spawn(ledger::websocket::update::subscribe(
        ledger::websocket::update::Params {
            ledger_host,
            party,
            filter: holding_filter(),
            access_token: token,
            ledger_end: begin,
        },
        on_update,
    ));
    tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_secs(15)))
        .await
        .map_err(|e| e.to_string())?;

    if subscription.is_finished() {
        let outcome = subscription.await.map_err(|e| e.to_string())?;
        println!("update subscription closed early: {outcome:?}");
    } else {
        subscription.abort();
        println!("update subscription stayed open for 15s");
    }
    println!(
        "update messages received from offset {begin}: {}",
        UPDATE_MESSAGES.load(Ordering::SeqCst)
    );

    Ok(())
}
