/// Example: Check and consolidate UTXOs if needed
///
/// Run with: cargo run -p examples --bin consolidate_utxos
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
        url: keycloak::login::password_url(
            &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
            &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
        ),
    };

    let auth = keycloak::login::password(login_params)
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

    let party = env::var("PARTY_ID").expect("PARTY_ID must be set");

    // You can customize the threshold (default is 10)
    let threshold: usize = env::var("CONSOLIDATION_THRESHOLD")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .expect("CONSOLIDATION_THRESHOLD must be a valid number");

    println!("\n🔄 Checking UTXO consolidation for party:");
    println!("   Party: {}", party);
    println!("   Threshold: {} UTXOs\n", threshold);

    let consolidate_params = cbtc::consolidate::CheckConsolidateParams {
        party,
        threshold,
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        access_token: auth.access_token,
        registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
        decentralized_party_id: env::var("DECENTRALIZED_PARTY_ID")
            .expect("DECENTRALIZED_PARTY_ID must be set"),
    };

    let result = cbtc::consolidate::check_and_consolidate(consolidate_params).await?;

    println!();
    if result.consolidated {
        println!("✅ Consolidation complete!");
        println!("   Before: {} UTXOs", result.utxos_before);
        println!("   After:  {} UTXO(s)", result.utxos_after);
        println!();
        println!("   Resulting holding CIDs:");
        for cid in &result.holding_cids {
            let short_id = if cid.len() > 16 {
                format!("{}...{}", &cid[..8], &cid[cid.len() - 8..])
            } else {
                cid.clone()
            };
            println!("     - {}", short_id);
        }
    } else {
        println!("✅ No consolidation needed");
        println!("   Current UTXO count: {}", result.utxos_before);
        println!("   Threshold: {}", threshold);
    }

    Ok(())
}
