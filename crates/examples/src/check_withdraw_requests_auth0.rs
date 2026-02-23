/// Check Withdraw Requests Example (Auth0 Version)
///
/// This example continuously polls for WithdrawRequests that have been created
/// by the attestor network after a user submitted a withdrawal.
///
/// Flow:
/// 1. User calls submit_withdraw() to burn CBTC (increases pending_balance)
/// 2. Attestor network processes the pending balance and creates a WithdrawRequest
/// 3. This script polls every 5 seconds to see processed withdrawals
///
/// The WithdrawRequest includes the btc_tx_id which is the Bitcoin transaction
/// that was used to fulfill the withdrawal.
///
/// To run this example:
/// 1. Make sure you have .env configured with AUTH0 credentials
/// 2. Submit a withdrawal first using redeem_cbtc_auth0
/// 3. cargo run --example check_withdraw_requests_auth0
/// 4. Press Ctrl+C to stop
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use cbtc::mint_redeem::redeem::{ListWithdrawAccountsParams, ListWithdrawRequestsParams};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== Check Withdraw Requests (Polling Mode) - Auth0 ===");
    println!("Press Ctrl+C to stop\n");

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

    let login_response = client_credentials(auth_params)
        .await
        .map_err(|e| format!("Auth0 authentication failed: {}", e))?;

    println!("✓ Authenticated successfully\n");

    // Common parameters
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    let mut poll_count = 0u64;

    loop {
        poll_count += 1;
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        println!("─────────────────────────────────────────────────────");
        println!("[{}] Poll #{}", timestamp, poll_count);
        println!("─────────────────────────────────────────────────────");

        // Check withdraw accounts for pending balances
        match cbtc::mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await
        {
            Ok(accounts) => {
                if accounts.is_empty() {
                    println!("No withdraw accounts found.");
                } else {
                    println!("Withdraw Accounts ({}):", accounts.len());
                    for account in &accounts {
                        let pending: f64 = account.pending_balance.parse().unwrap_or(0.0);
                        let status = if pending > 0.0 { "PENDING" } else { "ready" };
                        println!(
                            "  [{:>7}] {} BTC -> {}",
                            status, account.pending_balance, &account.destination_btc_address
                        );
                    }
                }
            }
            Err(e) => {
                println!("Error fetching accounts: {}", e);
            }
        }

        // Check for withdraw requests
        match cbtc::mint_redeem::redeem::list_withdraw_requests(ListWithdrawRequestsParams {
            ledger_host: ledger_host.clone(),
            party: party_id.clone(),
            access_token: access_token.clone(),
        })
        .await
        {
            Ok(requests) => {
                if requests.is_empty() {
                    println!("No withdraw requests yet.");
                } else {
                    println!("\nWithdraw Requests ({}):", requests.len());
                    for request in &requests {
                        println!(
                            "  {} BTC -> {} (tx: {})",
                            request.amount, &request.destination_btc_address, &request.btc_tx_id
                        );
                    }
                }
            }
            Err(e) => {
                println!("Error fetching requests: {}", e);
            }
        }

        println!("\nNext poll in 5 seconds...\n");
        sleep(Duration::from_secs(5)).await;
    }
}

