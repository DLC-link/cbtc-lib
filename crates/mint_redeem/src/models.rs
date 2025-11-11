use canton_api_client::models::JsActiveContract;
use serde::{Deserialize, Serialize};

/// Information about a contract (template ID, contract ID, and created event blob)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub contract_id: String,
    pub template_id: String,
    pub created_event_blob: String,
}

/// Account contract rules returned from attestor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountContractRuleSet {
    pub da_rules: ContractInfo, // DepositAccountRules
    pub wa_rules: ContractInfo, // WithdrawAccountRules
}

/// Token standard contracts returned from attestor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStandardContracts {
    pub burn_mint_factory: ContractInfo,
    pub instrument_configuration: ContractInfo,
    pub issuer_credential: Option<ContractInfo>,
    pub app_reward_configuration: Option<ContractInfo>,
    pub featured_app_right: Option<ContractInfo>,
}

/// A deposit account contract with its details
#[derive(Debug, Clone)]
pub struct DepositAccount {
    pub contract_id: String,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    pub last_processed_bitcoin_block: i64,
}

impl DepositAccount {
    /// Parse a DepositAccount from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();

        // Extract fields from createArgument
        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let owner = args
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'owner' field")?
            .to_string();

        let operator = args
            .get("operator")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'operator' field")?
            .to_string();

        let registrar = args
            .get("registrar")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'registrar' field")?
            .to_string();

        let last_processed_bitcoin_block = args
            .get("lastProcessedBitcoinBlock")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("Missing or invalid 'lastProcessedBitcoinBlock' field")?;

        Ok(Self {
            contract_id,
            owner,
            operator,
            registrar,
            last_processed_bitcoin_block,
        })
    }
}

/// A deposit request contract representing a completed BTC deposit
#[derive(Debug, Clone)]
pub struct DepositRequest {
    pub contract_id: String,
    pub deposit_account_id: String,
    pub amount: String,
    pub btc_tx_id: String,
}

impl DepositRequest {
    /// Parse a DepositRequest from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();

        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let deposit_account_id = args
            .get("depositAccountId")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'depositAccountId' field")?
            .to_string();

        let amount = args
            .get("amount")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'amount' field")?
            .to_string();

        let btc_tx_id = args
            .get("btcTxId")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'btcTxId' field")?
            .to_string();

        Ok(Self {
            contract_id,
            deposit_account_id,
            amount,
            btc_tx_id,
        })
    }
}

/// Status of a deposit account including Bitcoin address
#[derive(Debug, Clone)]
pub struct DepositAccountStatus {
    pub contract_id: String,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    pub bitcoin_address: String,
    pub last_processed_bitcoin_block: i64,
}
