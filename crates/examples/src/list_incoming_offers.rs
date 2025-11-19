/// Example: List Incoming CBTC Offers
///
/// This example lists all pending CBTC transfer offers where you are the receiver.
/// Use this to see what transfers are waiting for you to accept.
///
/// Run with: cargo run -p examples --bin list_incoming_offers
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load configuration from environment
    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");

    let keycloak_client_id =
        env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set");
    let keycloak_username = env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set");
    let keycloak_password = env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set");
    let keycloak_url = keycloak::login::password_url(
        &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
        &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
    );

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Incoming CBTC Transfer Offers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Receiver (you): {}", party);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Authenticate
    println!("Authenticating...");
    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: keycloak_client_id,
        username: keycloak_username,
        password: keycloak_password,
        url: keycloak_url,
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    let transfers =
        cbtc::utils::fetch_incoming_transfers(ledger_host, party.clone(), auth.access_token)
            .await?;

    if transfers.is_empty() {
        println!("No pending incoming transfers found.\n");
        return Ok(());
    }

    println!(
        "\nFound {} pending incoming transfer(s):\n",
        transfers.len()
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for (idx, transfer) in transfers.iter().enumerate() {
        let contract_id = &transfer.created_event.contract_id;
        let short_id = if contract_id.len() > 16 {
            format!(
                "{}...{}",
                &contract_id[..8],
                &contract_id[contract_id.len() - 8..]
            )
        } else {
            contract_id.clone()
        };

        println!("\n{}. Contract ID: {}", idx + 1, short_id);
        println!("   Full ID: {}", contract_id);

        // Extract transfer details
        if let Some(Some(create_arg)) = &transfer.created_event.create_argument {
            if let Some(transfer_data) = create_arg.get("transfer") {
                if let Some(sender) = transfer_data.get("sender") {
                    println!("   From: {}", sender.as_str().unwrap_or("unknown"));
                }
                if let Some(amount) = transfer_data.get("amount") {
                    println!("   Amount: {} CBTC", amount.as_str().unwrap_or("unknown"));
                }
                if let Some(requested_at) = transfer_data.get("requestedAt") {
                    println!(
                        "   Requested: {}",
                        requested_at.as_str().unwrap_or("unknown")
                    );
                }
                if let Some(execute_before) = transfer_data.get("executeBefore") {
                    println!(
                        "   Expires: {}",
                        execute_before.as_str().unwrap_or("unknown")
                    );
                }
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total: {} incoming offer(s)", transfers.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("Use 'accept_transfers' example to accept these offers");

    Ok(())
}
