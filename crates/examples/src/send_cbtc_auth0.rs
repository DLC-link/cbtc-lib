/// Example: Send CBTC to another party (Auth0 Version)
///
/// Run with: cargo run --example send_cbtc_auth0
///
/// Make sure to set up your .env file with:
/// - AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_CLIENT_SECRET, AUTH0_AUDIENCE
/// - LEDGER_HOST, PARTY_ID, REGISTRY_URL, DECENTRALIZED_PARTY_ID
/// - LIB_TEST_RECEIVER_PARTY_ID (the party to send CBTC to)
/// - TRANSFER_AMOUNT (optional, default: 0.00001)
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== Send CBTC Example (Auth0) ===\n");

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

    println!("✓ Authenticated successfully!");
    println!("  Token expires in: {} seconds\n", auth.expires_in);

    // Set up transfer parameters
    let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let receiver_party = env::var("LIB_TEST_RECEIVER_PARTY_ID")
        .expect("LIB_TEST_RECEIVER_PARTY_ID must be set (the party to send CBTC to)");
    let amount = env::var("TRANSFER_AMOUNT").unwrap_or_else(|_| "0.00001".to_string());

    println!("Transfer Details:");
    println!("  Amount: {} CBTC", amount);
    println!("  From: {}", sender_party);
    println!("  To: {}\n", receiver_party);

    // Check if receiver looks like a party ID (should contain ::)
    if !receiver_party.contains("::") {
        return Err(format!(
            "ERROR: Receiver '{}' does not look like a Canton party ID.\n\
            Party IDs should be in format: party-name::1220...\n\
            You provided what looks like a Bitcoin address. For sending CBTC,\n\
            you need a Canton party ID, not a Bitcoin address.",
            receiver_party
        ));
    }

    // Check balance before attempting transfer
    println!("Checking CBTC balance...");
    let holdings = cbtc::active_contracts::get(cbtc::active_contracts::Params {
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        party: sender_party.clone(),
        access_token: auth.access_token.clone(),
    })
    .await?;

    let total_balance: f64 = holdings.iter().filter_map(cbtc::utils::extract_amount).sum();

    println!("  Current balance: {:.8} CBTC", total_balance);
    println!("  UTXO count: {}\n", holdings.len());

    if holdings.is_empty() {
        return Err(format!(
            "ERROR: No CBTC holdings found for party {}.\n\
            You need to have CBTC tokens before you can send them.\n\
            \n\
            To get CBTC:\n\
            1. Run 'cargo run --example mint_cbtc_auth0' to create a deposit account\n\
            2. Send BTC to the Bitcoin address provided\n\
            3. Wait for CBTC to be minted (check balance again)",
            sender_party
        ));
    }

    let amount_f64: f64 = amount.parse().map_err(|e| format!("Invalid amount: {}", e))?;
    if total_balance < amount_f64 {
        return Err(format!(
            "ERROR: Insufficient balance.\n\
            You have {:.8} CBTC but trying to send {} CBTC",
            total_balance, amount
        ));
    }

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
    println!("Submitting transfer...");
    cbtc::transfer::submit(transfer_params).await?;

    println!("✅ Transfer submitted successfully!");
    println!("\nNote: The receiver must accept the transfer for it to complete.");

    Ok(())
}

