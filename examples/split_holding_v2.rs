/// Example: Split a holding into several, Token Standard V2
///
/// Run with: cargo run --example split_holding_v2
///
/// Splitting turns one holding into several of the amounts you name, plus
/// change. This example reads the party's holdings, picks the largest, and
/// splits it. Set SPLIT_AMOUNTS to a comma-separated list to choose the
/// outputs; it defaults to one output of 0.001 CBTC.
///
/// V2 takes a `cbtc::Account` where V1 takes the party as a string.
/// `Account::basic` builds the account every party owns, with no provider
/// and an empty id.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("Authenticating...");
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
    .map_err(|e| format!("Authentication failed: {}", e))?;

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let decentralized_party_id = shared::resolve_party_id();
    let instrument = cbtc::InstrumentId {
        admin: decentralized_party_id.clone(),
        id: cbtc::CBTC_TICKER.to_string(),
    };

    let amounts: Vec<cbtc::DamlDecimal> = env::var("SPLIT_AMOUNTS")
        .unwrap_or_else(|_| "0.001".to_string())
        .split(',')
        .map(|part| {
            cbtc::DamlDecimal::parse(part.trim())
                .unwrap_or_else(|e| panic!("SPLIT_AMOUNTS holds an invalid amount: {e}"))
        })
        .collect();

    println!("\n✂️  Splitting a holding (V2)");
    println!("   Party: {}", party);

    // Read the same account the split names below. `account: None` returns
    // every account the party owns, and a holding under a labelled account
    // cannot be split from the basic one: the registry rejects the mismatch.
    let holdings = cbtc::active_contracts::get(cbtc::active_contracts::Params {
        ledger_host: ledger_host.clone(),
        party: party.clone(),
        access_token: auth.access_token.clone(),
        instrument_id: instrument.clone(),
        account: Some(cbtc::Account::basic(party.clone())),
    })
    .await?;

    // The largest holding is the one most likely to cover the requested
    // amounts, and splitting it leaves the smaller ones untouched.
    let (largest, amount) = holdings
        .iter()
        .filter_map(|holding| cbtc::utils::extract_amount(holding).map(|a| (holding, a)))
        .max_by(|left, right| left.1.cmp(&right.1))
        .ok_or("the party's basic account holds no CBTC, so there is nothing to split")?;

    let input_cid = largest.created_event.contract_id.clone();
    println!("   Input:  {} CBTC ({})", amount, input_cid);
    println!(
        "   Outputs: {}",
        amounts
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let result = cbtc::split::v2::submit(cbtc::split::v2::Params {
        account: cbtc::Account::basic(party),
        amounts,
        instrument_id: instrument,
        input_holding_cids: vec![input_cid],
        ledger_host,
        access_token: auth.access_token,
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id,
    })
    .await
    .map_err(|e| {
        // A failed split reports what it did create, so the caller can
        // reconcile rather than guess.
        format!(
            "{} | created {} output(s) and {} change holding(s) before failing",
            e.message,
            e.partial.output_holding_cids.len(),
            e.partial.change_holding_cids.len()
        )
    })?;

    println!("\n✅ Split complete.");
    println!("   Outputs:");
    for cid in &result.output_holding_cids {
        println!("     - {}", cid);
    }
    println!("   Change:");
    for cid in &result.change_holding_cids {
        println!("     - {}", cid);
    }

    Ok(())
}
