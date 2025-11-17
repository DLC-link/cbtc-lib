/// Example: List Outgoing CBTC Offers
///
/// This example lists all pending CBTC transfer offers where you are the sender.
/// Use this to see what transfers you've sent that haven't been accepted yet.
///
/// Run with: cargo run -p examples --example list_outgoing_offers
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();

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
    println!("Outgoing CBTC Transfer Offers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Sender (you): {}", party);
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

    // Fetch outgoing transfers
    let transfers =
        cbtc::utils::fetch_outgoing_transfers(ledger_host, party.clone(), auth.access_token)
            .await?;

    if transfers.is_empty() {
        println!("No pending outgoing transfers found.\n");
        return Ok(());
    }

    println!(
        "\nFound {} pending outgoing transfer(s):\n",
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
                if let Some(receiver) = transfer_data.get("receiver") {
                    println!("   To: {}", receiver.as_str().unwrap_or("unknown"));
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
    println!("Total: {} outgoing offer(s)", transfers.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("Use 'withdraw_transfers' example to cancel these offers");

    Ok(())
}
