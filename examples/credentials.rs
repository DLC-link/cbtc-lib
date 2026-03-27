use cbtc::credentials::{
    AcceptCredentialOfferParams, FindUserServiceParams, ListCredentialOffersParams,
    ListCredentialsParams,
};
use keycloak::login::{PasswordParams, password, password_url};
use std::env;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    println!("=== CBTC Credential Example ===\n");

    // Step 1: Authenticate with Keycloak
    println!("Step 1: Authenticating with Keycloak...");
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
    println!("  Authenticated successfully\n");

    let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
    let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
    let access_token = login_response.access_token.clone();

    // Step 2: Check for existing credentials
    println!("Step 2: Checking for existing credentials...");
    let credentials = cbtc::credentials::list_credentials(ListCredentialsParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    // Filter for CBTC Minter credentials
    let minter_credentials: Vec<_> = credentials
        .iter()
        .filter(|c| {
            c.claims
                .iter()
                .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
        })
        .collect();

    if !minter_credentials.is_empty() {
        println!("  Found {} Minter credential(s):", minter_credentials.len());
        for cred in &minter_credentials {
            println!("    - ID: {}, Contract: {}", cred.id, cred.contract_id);
            println!("      Issuer: {}", cred.issuer);
            for claim in &cred.claims {
                println!(
                    "      Claim: {}.{} = {}",
                    claim.subject, claim.property, claim.value
                );
            }
        }
        println!();
        println!("=== Example Complete ===");
        println!(
            "  Use credential CID {} in CBTC operations.",
            minter_credentials[0].contract_id
        );
        return Ok(());
    }

    println!("  No Minter credentials found.\n");

    // Step 3: Check for pending credential offers
    println!("Step 3: Checking for pending credential offers...");
    let offers = cbtc::credentials::list_credential_offers(ListCredentialOffersParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;

    if offers.is_empty() {
        println!("  No credential offers found.");
        println!("  The attestor network must offer you a credential before you can accept one.");
        println!("  Contact your CBTC operator to request credential issuance.");
        return Ok(());
    }

    println!("  Found {} credential offer(s):", offers.len());
    for offer in &offers {
        println!("    - ID: {}, Contract: {}", offer.id, offer.contract_id);
        println!("      Issuer: {}", offer.issuer);
        println!("      Description: {}", offer.description);
    }
    println!();

    // Step 4: Find UserService contract
    println!("Step 4: Finding UserService contract...");
    let user_service = cbtc::credentials::find_user_service(FindUserServiceParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
    })
    .await?;
    println!("  Found UserService: {}\n", user_service.contract_id);

    // Step 5: Accept the first offer
    let offer = &offers[0];
    println!("Step 5: Accepting credential offer '{}'...", offer.id);
    let credential = cbtc::credentials::accept_credential_offer(AcceptCredentialOfferParams {
        ledger_host: ledger_host.clone(),
        party: party_id.clone(),
        access_token: access_token.clone(),
        user_service_contract_id: user_service.contract_id.clone(),
        user_service_template_id: user_service.template_id.clone(),
        credential_offer_cid: offer.contract_id.clone(),
    })
    .await?;

    println!("  Credential accepted!");
    println!("    Contract ID: {}", credential.contract_id);
    println!("    ID: {}", credential.id);
    for claim in &credential.claims {
        println!(
            "    Claim: {}.{} = {}",
            claim.subject, claim.property, claim.value
        );
    }
    println!();

    println!("=== Example Complete ===");
    println!(
        "  Use credential CID {} in CBTC operations (e.g., create_deposit_account, submit_withdraw).",
        credential.contract_id
    );

    Ok(())
}
