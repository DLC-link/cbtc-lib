/// Example: Withdraw a DvP allocation and unlock the holdings
///
/// Run with: ALLOCATION_CONTRACT_ID=<cid> cargo run --example withdraw_allocation
///
/// `allocate_cbtc` locks holdings into a settlement leg. Until the executor
/// settles that leg, the sender takes the holdings back by withdrawing the
/// allocation. This example does that, and `cancel` works the same way for
/// the executor.
///
/// You supply the allocation's contract id, because nothing in the crate
/// lists allocations. `allocate_cbtc` prints the id when it succeeds, along
/// with the command that reclaims it, so copy that line. Issue #85 tracks
/// the listing itself.
use std::env;
mod shared;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let contract_id = env::var("ALLOCATION_CONTRACT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(
            "ALLOCATION_CONTRACT_ID must be set to the allocation to withdraw. \
             Run allocate_cbtc, which prints the id and the command that \
             reclaims it.",
        )?;

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

    println!("\n🔓 Withdrawing a DvP allocation");
    println!("   Party:      {}", party);
    println!("   Allocation: {}", contract_id);

    cbtc::allocation::withdraw(cbtc::allocation::ActionParams {
        allocation_contract_id: contract_id,
        actor_party: party,
        ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
        access_token: auth.access_token,
        registry_url: shared::resolve_registry_url(),
        decentralized_party_id: shared::resolve_party_id(),
    })
    .await?;

    println!("\n✅ Allocation withdrawn. The locked holdings return to the sender.");

    Ok(())
}
