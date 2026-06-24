/// Example: Allocate CBTC into a DvP settlement leg.
///
/// Locks the sender's CBTC into one leg of a Delivery-versus-Payment settlement
/// via `AllocationFactory_Allocate`. The settlement executor (venue) later
/// settles all legs atomically before `settleBefore`.
///
/// Run with: cargo run --example allocate_cbtc
///
/// Make sure to set up your .env file with the required configuration.
use std::env;

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
        // The CBTC Keycloak uses the legacy `/auth` context root, so pass
        // `{host}/auth` to the (non-deprecated) `token_url` helper.
        url: keycloak::login::token_url(
            &format!(
                "{}/auth",
                env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set")
            ),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    let auth = keycloak::login::password(login_params)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

    println!("Authenticated successfully!");

    // Settlement participants
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let receiver_party = env::var("LIB_TEST_RECEIVER_PARTY_ID")
        .expect("LIB_TEST_RECEIVER_PARTY_ID must be set (the leg receiver)");
    let executor_party = env::var("EXECUTOR_PARTY_ID")
        .expect("EXECUTOR_PARTY_ID must be set (the settlement executor / venue)");
    let amount_str = env::var("ALLOCATE_AMOUNT").unwrap_or_else(|_| "0.1".to_string());
    let amount = cbtc::DamlDecimal::parse(&amount_str).expect("Invalid ALLOCATE_AMOUNT");
    let decentralized_party =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");
    let settlement_ref_id =
        env::var("SETTLEMENT_REF_ID").unwrap_or_else(|_| "cbtc-dvp-example".to_string());

    // Allocation must be funded before `allocateBefore` and settled before
    // `settleBefore` (which must be later).
    let now = chrono::Utc::now();
    let allocate_before = now
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .to_rfc3339();
    let settle_before = now
        .checked_add_signed(chrono::Duration::hours(48))
        .unwrap()
        .to_rfc3339();

    println!("\nAllocating {} CBTC into settlement leg", amount);
    println!("Sender:   {}", sender_party);
    println!("Receiver: {}", receiver_party);
    println!("Executor: {}", executor_party);

    let allocation = common::allocation::AllocationSpecification {
        settlement: common::allocation::SettlementInfo {
            executor: executor_party,
            settlement_ref: common::allocation::Reference {
                id: settlement_ref_id,
                cid: None,
            },
            requested_at: now.to_rfc3339(),
            allocate_before,
            settle_before,
            meta: common::allocation::Metadata::default(),
        },
        transfer_leg_id: "leg0".to_string(),
        transfer_leg: common::allocation::TransferLeg {
            sender: sender_party,
            receiver: receiver_party,
            amount,
            instrument_id: common::transfer::InstrumentId {
                admin: decentralized_party.clone(),
                id: "CBTC".to_string(),
            },
            meta: common::allocation::Metadata::default(),
        },
    };

    let params = cbtc::allocation::AllocateParams {
        allocation,
        requested_at: now.to_rfc3339(),
        input_holding_cids: Vec::new(), // Library auto-selects the sender's holdings
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        access_token: auth.access_token,
        registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
        decentralized_party_id: decentralized_party,
    };

    // Submit allocation
    println!("\nSubmitting allocation...");
    cbtc::allocation::allocate(params).await?;

    println!("✅ Allocation submitted successfully!");
    println!(
        "\nNote: the executor settles all legs of the settlement atomically before settleBefore."
    );
    println!("To reclaim before settlement, withdraw the allocation as the sender.");

    Ok(())
}
