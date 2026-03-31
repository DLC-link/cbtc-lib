use crate::mint_redeem::models::{
    AccountContractRuleSet, BitcoinAddressResponse, TokenStandardContracts,
};

/// Get the Bitcoin address for a deposit or withdraw account
///
/// # Arguments
/// * `api_url` - Base URL of the Bitsafe API (e.g., "https://api.bitsafe.finance")
/// * `account_id` - The account ID (UUID when present, otherwise contract ID) of the deposit/withdraw account
///
/// # Returns
/// The Bitcoin address associated with this account
///
/// # Example
/// ```ignore
/// let bitcoin_address = get_bitcoin_address(
///     "https://api.bitsafe.finance",
///     "00febb6b97f5d214bb..."
/// ).await?;
/// println!("BTC address: {}", bitcoin_address);
/// ```
pub async fn get_bitcoin_address(api_url: &str, account_id: &str) -> Result<String, String> {
    let url = format!("{}/cbtc/v1/bitcoin-address/{}", api_url, account_id);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Bitsafe API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Bitsafe API returned error status: {}",
            response.status()
        ));
    }

    let bitcoin_address_response: BitcoinAddressResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(bitcoin_address_response.bitcoin_address)
}

/// Get the account contract rules from the Bitsafe API
///
/// # Arguments
/// * `api_url` - Base URL of the Bitsafe API
///
/// # Returns
/// Account contract rule set containing DepositAccountRules and WithdrawAccountRules
///
/// # Example
/// ```ignore
/// let rules = get_account_contract_rules(
///     "https://api.bitsafe.finance"
/// ).await?;
/// ```
pub async fn get_account_contract_rules(api_url: &str) -> Result<AccountContractRuleSet, String> {
    let url = format!("{}/cbtc/v1/account-contract-rules", api_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Bitsafe API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Bitsafe API returned error status: {}",
            response.status()
        ));
    }

    let rules: AccountContractRuleSet = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(rules)
}

/// Get the token standard contracts from the Bitsafe API
///
/// # Arguments
/// * `api_url` - Base URL of the Bitsafe API
///
/// # Returns
/// Token standard contracts including burn_mint_factory, instrument_configuration, etc.
///
/// # Example
/// ```ignore
/// let contracts = get_token_standard_contracts(
///     "https://api.bitsafe.finance"
/// ).await?;
/// ```
pub async fn get_token_standard_contracts(api_url: &str) -> Result<TokenStandardContracts, String> {
    let url = format!("{}/cbtc/v1/token-standard-contracts", api_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Bitsafe API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Bitsafe API returned error status: {}",
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

        let api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");

        let rules = get_account_contract_rules(&api_url)
            .await
            .expect("Failed to get account contract rules");

        assert!(!rules.da_rules.contract_id.is_empty());
        assert!(!rules.wa_rules.contract_id.is_empty());
    }

    #[tokio::test]
    async fn test_get_token_standard_contracts() {
        dotenvy::dotenv().ok();

        let api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");

        let contracts = get_token_standard_contracts(&api_url)
            .await
            .expect("Failed to get token standard contracts");

        assert!(!contracts.burn_mint_factory.contract_id.is_empty());
        assert!(!contracts.instrument_configuration.contract_id.is_empty());
        assert!(!contracts.issuer_credential.contract_id.is_empty());
    }
}
