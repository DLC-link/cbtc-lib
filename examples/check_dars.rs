/// Example: Check DAR packages on participant
///
/// Verifies that all required DAR packages are uploaded to the participant node
/// by scanning the DAR files in cbtc-dars/ and comparing against the participant.
///
/// Run with: cargo run --example check_dars
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST
use std::env;
use std::process;

#[tokio::main]
async fn main() {
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
        .unwrap_or_else(|e| {
            eprintln!("Authentication failed: {}", e);
            process::exit(1);
        });

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");

    println!("Checking DAR packages on participant...");
    println!("  Ledger host: {}", ledger_host);
    println!();

    let params = cbtc::dar_check::Params {
        ledger_host,
        access_token: auth.access_token,
        dar_dirs: vec![
            "cbtc-dars/dars/dependencies".to_string(),
            "cbtc-dars/dars/cbtc".to_string(),
        ],
    };

    let result = cbtc::dar_check::check(params).await.unwrap_or_else(|e| {
        eprintln!("DAR check failed: {}", e);
        process::exit(1);
    });

    // Print results
    println!(
        "Found {}/{} expected packages",
        result.found.len(),
        result.total_expected
    );
    println!();

    if !result.missing.is_empty() {
        println!("Missing packages:");
        for info in &result.missing {
            println!("  {} v{} ({})", info.name, info.version, info.package_id);
        }
        println!();
    }

    match result.status {
        cbtc::dar_check::DarCheckStatus::Pass => {
            println!("PASS: All required DAR packages are present.");
        }
        cbtc::dar_check::DarCheckStatus::Fail => {
            println!(
                "FAIL: {} packages are missing.",
                result.missing.len()
            );
            println!();
            println!("Note: Missing packages may not yet be required for your environment.");
            println!("This repo may include DARs ahead of what is deployed on Canton Network mainnet.");
            println!();
            println!("To verify which versions are required for your environment:");
            println!("  Splice DARs:  https://github.com/hyperledger-labs/splice/tree/main/daml/dars");
            println!("                (select the tag matching your environment release)");
            println!("  Utility DARs: https://docs.digitalasset.com/utilities/releases/index.html");
            println!();
            println!("To upload missing DARs to your participant: cbtc-dars/upload_dars.sh");
            process::exit(1);
        }
    }
}
