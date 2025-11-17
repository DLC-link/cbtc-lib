/// Monitor for CBTC deposit requests
///
/// This example continuously monitors for deposit requests and prints them when found.
///
/// Usage:
/// cargo run --example monitor_deposits
use keycloak::login::{password, password_url, PasswordParams};
use mint_redeem::mint::ListDepositRequestsParams;
use std::env;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Deposit Monitor ===\n");

    // Authenticate
    println!("Authenticating...");
    let params = PasswordParams {
        client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
        username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
        url: password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };
    let login_response = password(params).await?;
    println!("✓ Authenticated\n");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");

    println!("Monitoring for deposit requests (checking every 10 seconds)...");
    println!("Press Ctrl+C to stop\n");

    let mut last_count = 0;

    loop {
        match mint_redeem::mint::list_deposit_requests(ListDepositRequestsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: login_response.access_token.clone(),
        })
        .await
        {
            Ok(requests) => {
                if requests.len() != last_count {
                    println!("\n✓ Found {} deposit request(s):", requests.len());
                    for (i, request) in requests.iter().enumerate() {
                        println!("  {}. Deposit Request:", i + 1);
                        println!("     Contract ID: {}", request.contract_id);
                        println!("     Deposit Account: {}", request.deposit_account_id);
                        println!("     Amount: {} BTC", request.amount);
                        println!("     BTC TX ID: {}", request.btc_tx_id);
                        println!();
                    }
                    last_count = requests.len();
                } else if !requests.is_empty() {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
            }
            Err(e) => {
                // Ignore 404 errors (template doesn't exist yet)
                if !e.contains("404") {
                    eprintln!("Error checking deposits: {}", e);
                }
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}
