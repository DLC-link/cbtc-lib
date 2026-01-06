use cbtc::mint_redeem;
/// Test burning CBTC using an existing withdraw account
///
/// This example demonstrates burning a small amount of CBTC using an existing
/// withdraw account instead of creating a new one.
///
/// Usage:
/// cargo run -p examples --bin test_burn_cbtc
use keycloak::login::{PasswordParams, password, password_url};
use mint_redeem::redeem::{ListHoldingsParams, ListWithdrawAccountsParams, SubmitWithdrawParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

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

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();
    let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
    let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

    let accounts = mint_redeem::redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let my_accounts: Vec<_> = accounts.iter().filter(|a| a.owner == party_id).collect();

    if my_accounts.is_empty() {
        return Err(
            "No withdraw accounts found. Run 'redeem_cbtc_flow' example first.".to_string(),
        );
    }

    let withdraw_account = my_accounts[0];

    let holdings = mint_redeem::redeem::list_holdings(ListHoldingsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    let cbtc_holdings: Vec<_> = holdings
        .iter()
        .filter(|h| h.instrument_id == "CBTC" && h.owner == party_id)
        .collect();

    if cbtc_holdings.is_empty() {
        return Err("No CBTC holdings found to burn".to_string());
    }

    let burn_amount = "0.0001";
    let burn_amount_f64: f64 = burn_amount.parse().unwrap();

    let mut selected_holdings = Vec::new();
    let mut selected_total = 0.0;

    for holding in &cbtc_holdings {
        let amount = holding.amount.parse::<f64>().unwrap_or(0.0);
        selected_holdings.push(holding.contract_id.clone());
        selected_total += amount;
        if selected_total >= burn_amount_f64 {
            break;
        }
    }

    if selected_total < burn_amount_f64 {
        return Err(format!(
            "Insufficient balance. Have {}, need {}",
            selected_total, burn_amount
        ));
    }

    let updated_account = mint_redeem::redeem::submit_withdraw(SubmitWithdrawParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
        access_token: access_token.clone(),
        attestor_url: attestor_url.clone(),
        chain: chain.clone(),
        withdraw_account_contract_id: withdraw_account.contract_id.clone(),
        withdraw_account_template_id: withdraw_account.template_id.clone(),
        withdraw_account_created_event_blob: withdraw_account.created_event_blob.clone(),
        amount: burn_amount.to_string(),
        holding_contract_ids: selected_holdings,
    })
    .await?;

    println!(
        "Burn successful! Pending balance: {} BTC",
        updated_account.pending_balance
    );

    Ok(())
}
