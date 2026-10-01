/// Example: Reject incoming CBTC transfer offers
///
/// Run with: cargo run --example reject_transfer
///
/// Rejecting is the receiver's action, so this example authenticates as the
/// receiver rather than the sender. It reads `RECEIVER_PARTY_ID` and the
/// `RECEIVER_KEYCLOAK_*` variables, falling back to the sender's host and
/// realm where the receiver shares them.
///
/// Token Standard V2 needs no separate example. `reject::v2` re-exports V1's
/// `Params` unchanged, so calling `cbtc::reject::v2::submit` with the same
/// arguments is the whole difference.
///
/// Create an offer to reject by running `send_cbtc` first. A transfer to
/// another party creates an offer and waits. A transfer to your own party
/// settles on submission and leaves nothing to reject. `send_cbtc` sends to
/// `LIB_TEST_RECEIVER_PARTY_ID`, and this example rejects as
/// `RECEIVER_PARTY_ID`, so both must name this receiver.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // An empty override is not an override: a `.env` assigns rather than unsets.
    let receiver_var = |name: &str| env::var(name).ok().filter(|s| !s.is_empty());

    let party = receiver_var("RECEIVER_PARTY_ID")
        .expect("RECEIVER_PARTY_ID must be set to the party that rejects the offers");
    let ledger_host = receiver_var("RECEIVER_LEDGER_HOST")
        .unwrap_or_else(|| env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"));
    let keycloak_url = keycloak::login::token_url(
        &receiver_var("RECEIVER_KEYCLOAK_HOST")
            .unwrap_or_else(|| env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set")),
        &receiver_var("RECEIVER_KEYCLOAK_REALM")
            .unwrap_or_else(|| env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set")),
    );

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Reject incoming CBTC transfer offers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Receiver (you): {}", party);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Authenticating...");
    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: env::var("RECEIVER_KEYCLOAK_CLIENT_ID")
            .expect("RECEIVER_KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("RECEIVER_KEYCLOAK_USERNAME")
            .expect("RECEIVER_KEYCLOAK_USERNAME must be set"),
        password: env::var("RECEIVER_KEYCLOAK_PASSWORD")
            .expect("RECEIVER_KEYCLOAK_PASSWORD must be set"),
        url: keycloak_url,
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    let decentralized_party = shared::resolve_party_id();
    let instrument = cbtc::InstrumentId {
        admin: decentralized_party.clone(),
        id: cbtc::CBTC_TICKER.to_string(),
    };

    let transfers = cbtc::utils::fetch_incoming_transfers(
        ledger_host.clone(),
        party.clone(),
        auth.access_token.clone(),
        instrument,
    )
    .await?;

    if transfers.is_empty() {
        println!("No pending incoming transfers found.");
        println!("Run `cargo run --example send_cbtc` from the sender to create one.");
        println!(
            "It sends to LIB_TEST_RECEIVER_PARTY_ID, which must name the same party as \
             RECEIVER_PARTY_ID.\n"
        );
        return Ok(());
    }

    println!("Found {} pending offer(s).\n", transfers.len());

    let mut rejected = 0;
    for (idx, transfer) in transfers.iter().enumerate() {
        let contract_id = &transfer.created_event.contract_id;
        println!("{}. Rejecting {}", idx + 1, contract_id);

        if let Some(create_arg) = &transfer.created_event.create_argument
            && let Some(data) = create_arg.get("transfer")
        {
            if let Some(sender) = data.get("sender").and_then(|v| v.as_str()) {
                println!("   From:   {}", sender);
            }
            if let Some(amount) = data.get("amount").and_then(|v| v.as_str()) {
                println!("   Amount: {} CBTC", amount);
            }
        }

        cbtc::reject::submit(cbtc::reject::Params {
            transfer_offer_contract_id: contract_id.clone(),
            receiver_party: party.clone(),
            ledger_host: ledger_host.clone(),
            access_token: auth.access_token.clone(),
            registry_url: shared::resolve_registry_url(),
            decentralized_party_id: decentralized_party.clone(),
        })
        .await?;

        rejected += 1;
        println!("   Rejected.\n");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Rejected {} offer(s).", rejected);
    println!("\nNote: the sender's holdings return to the sender.");

    Ok(())
}
