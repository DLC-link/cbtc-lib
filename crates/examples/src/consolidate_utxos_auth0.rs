/// Example: Check and consolidate UTXOs if needed (Auth0 Version)
///
/// Run with: cargo run --example consolidate_utxos_auth0
///
/// Make sure to set up your .env file with the required configuration.
use cbtc::auth0::{auth0_url, client_credentials, ClientCredentialsParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load environment variables
    dotenvy::dotenv().ok();
    env_logger::init();

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

