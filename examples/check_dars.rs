/// Example: Check DAR packages on participant
///
/// Verifies that all required DAR packages are uploaded to the participant node
/// by comparing against the expected packages manifest.
///
/// Run with: cargo run --example check_dars
///
/// Required environment variables:
/// - KEYCLOAK_HOST, KEYCLOAK_REALM, KEYCLOAK_CLIENT_ID
/// - KEYCLOAK_USERNAME, KEYCLOAK_PASSWORD
/// - LEDGER_HOST
///
/// Optional environment variables:
/// - DAR_MANIFEST_PATH (defaults to "cbtc-dars/expected_packages.json")
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
    let manifest_path = env::var("DAR_MANIFEST_PATH")
        .unwrap_or_else(|_| "cbtc-dars/expected_packages.json".to_string());

    println!("Checking DAR packages on participant...");
    println!("  Ledger host: {}", ledger_host);
    println!("  Manifest:    {}", manifest_path);
    println!();

    let params = cbtc::dar_check::Params {
        ledger_host,
        access_token: auth.access_token,
        manifest_path,
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
        for (pkg_id, info) in &result.missing {
            println!("  {} v{} ({})", info.name, info.version, pkg_id);
        }
        println!();
    }

    match result.status {
        cbtc::dar_check::DarCheckStatus::Pass => {
            println!("PASS: All required DAR packages are present.");
        }
        cbtc::dar_check::DarCheckStatus::Fail => {
            println!(
                "FAIL: {} packages are missing. Upload them using cbtc-dars/upload_dars.sh",
                result.missing.len()
            );
            process::exit(1);
        }
    }
}
