use crate::models::{AccountContractRuleSet, TokenStandardContracts};
use serde_json::json;

/// Get the Bitcoin address for a deposit or withdraw account
///
/// # Arguments
/// * `attestor_url` - Base URL of the attestor (e.g., "https://devnet.dlc.link/attestor-1")
/// * `account_id` - The UUID from the deposit/withdraw account contract's `id` field
/// * `chain` - The chain identifier (e.g., "canton-devnet", "canton-testnet")
///
/// # Returns
/// The Bitcoin address associated with this account
///
/// # Example
/// ```ignore
/// let bitcoin_address = get_bitcoin_address(
///     "https://devnet.dlc.link/attestor-1",
///     "550e8400-e29b-41d4-a716-446655440000",
///     "canton-devnet"
/// ).await?;
/// ```
pub async fn get_bitcoin_address(
    attestor_url: &str,
    account_id: &str,
    chain: &str,
) -> Result<String, String> {
    let url = format!("{}/app/get-bitcoin-address", attestor_url);

    let body = json!({
        "id": account_id,
        "chain": chain,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to attestor: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Attestor returned error status: {}",
            response.status()
        ));
    }

    let bitcoin_address = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(bitcoin_address)
}

/// Get the account contract rules from the attestor
///
/// # Arguments
/// * `attestor_url` - Base URL of the attestor
/// * `chain` - The chain identifier
///
/// # Returns
/// Account contract rule set containing DepositAccountRules and WithdrawAccountRules
///
/// # Example
/// ```ignore
/// let rules = get_account_contract_rules(
///     "https://devnet.dlc.link/attestor-1",
///     "canton-devnet"
/// ).await?;
/// ```
pub async fn get_account_contract_rules(
    attestor_url: &str,
    chain: &str,
) -> Result<AccountContractRuleSet, String> {
    let url = format!("{}/app/get-account-contract-rules", attestor_url);

    let body = json!({
        "chain": chain,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to attestor: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Attestor returned error status: {}",
            response.status()
        ));
    }

    let rules: AccountContractRuleSet = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(rules)
}

/// Get the token standard contracts from the attestor
///
/// # Arguments
/// * `attestor_url` - Base URL of the attestor
/// * `chain` - The chain identifier
///
/// # Returns
/// Token standard contracts including burn_mint_factory, instrument_configuration, etc.
///
/// # Example
/// ```ignore
/// let contracts = get_token_standard_contracts(
///     "https://devnet.dlc.link/attestor-1",
///     "canton-devnet"
/// ).await?;
/// ```
pub async fn get_token_standard_contracts(
    attestor_url: &str,
    chain: &str,
) -> Result<TokenStandardContracts, String> {
    let url = format!("{}/app/get-token-standard-contracts", attestor_url);

    let body = json!({
        "chain": chain,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to attestor: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Attestor returned error status: {}",
            response.status()
        ));
    }

    let contracts: TokenStandardContracts = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(contracts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_get_account_contract_rules() {
        dotenvy::dotenv().ok();

        let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
        let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

        let rules = get_account_contract_rules(&attestor_url, &chain)
            .await
            .expect("Failed to get account contract rules");

        println!("DepositAccountRules contract ID: {}", rules.da_rules.contract_id);
        println!("WithdrawAccountRules contract ID: {}", rules.wa_rules.contract_id);

        assert!(!rules.da_rules.contract_id.is_empty());
        assert!(!rules.wa_rules.contract_id.is_empty());
    }

    #[tokio::test]
    async fn test_get_token_standard_contracts() {
        dotenvy::dotenv().ok();

        let attestor_url = env::var("ATTESTOR_URL").expect("ATTESTOR_URL must be set");
        let chain = env::var("CANTON_NETWORK").expect("CANTON_NETWORK must be set");

        let contracts = get_token_standard_contracts(&attestor_url, &chain)
            .await
            .expect("Failed to get token standard contracts");

        println!("Burn/Mint Factory: {}", contracts.burn_mint_factory.contract_id);
        println!("Instrument Config: {}", contracts.instrument_configuration.contract_id);

        assert!(!contracts.burn_mint_factory.contract_id.is_empty());
        assert!(!contracts.instrument_configuration.contract_id.is_empty());
    }
}
