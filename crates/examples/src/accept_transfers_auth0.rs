/// Example: Accept all pending CBTC transfers (Auth0 Version)
///
/// Run with: cargo run --example accept_transfers_auth0
///
/// This example fetches and accepts all pending CBTC TransferInstruction contracts
/// for your party using Auth0 authentication.
///
/// Make sure to set up your .env file with AUTH0 credentials.
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== Accept All Pending CBTC Transfers (Auth0) ===\n");

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

    let receiver_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let registry_url = env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let decentralized_party_id =
        env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

    // Fetch pending transfers
    println!("Fetching pending incoming transfers...");
    let pending_transfers = cbtc::utils::fetch_incoming_transfers(
        ledger_host.clone(),
        receiver_party.clone(),
        auth.access_token.clone(),
    )
    .await?;

    if pending_transfers.is_empty() {
        println!("No pending transfers found.");
        return Ok(());
    }

    println!("Found {} pending transfer(s)\n", pending_transfers.len());

    // Accept each transfer
    let mut successful = 0;
    let mut failed = 0;

    for (idx, transfer) in pending_transfers.iter().enumerate() {
        let contract_id = &transfer.created_event.contract_id;
        println!(
            "[{}/{}] Accepting transfer: {}...",
            idx + 1,
            pending_transfers.len(),
            if contract_id.len() > 16 {
                &contract_id[..16]
            } else {
                contract_id
            }
        );

        // Accept the transfer
        match cbtc::accept::submit(cbtc::accept::Params {
            transfer_offer_contract_id: contract_id.clone(),
            receiver_party: receiver_party.clone(),
            ledger_host: ledger_host.clone(),
            access_token: auth.access_token.clone(),
            registry_url: registry_url.clone(),
            decentralized_party_id: decentralized_party_id.clone(),
        })
        .await
        {
            Ok(_) => {
                println!("  ✓ Accepted successfully");
                successful += 1;
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
                failed += 1;
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Acceptance Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);

    if failed > 0 {
        return Err(format!("Completed with {} failures", failed));
    }

    Ok(())
}

