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

/// A withdraw account contract with its details
#[derive(Debug, Clone)]
pub struct WithdrawAccount {
    pub contract_id: String,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    pub destination_btc_address: String,
}

impl WithdrawAccount {
    /// Parse a WithdrawAccount from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();

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

        let destination_btc_address = args
            .get("destinationBtcAddress")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'destinationBtcAddress' field")?
            .to_string();

        Ok(Self {
            contract_id,
            owner,
            operator,
            registrar,
            destination_btc_address,
        })
    }
}

/// A withdraw request contract representing a CBTC burn and pending BTC withdrawal
#[derive(Debug, Clone)]
pub struct WithdrawRequest {
    pub contract_id: String,
    pub withdraw_account_id: String,
    pub amount: String,
    pub destination_btc_address: String,
    pub btc_tx_id: Option<String>,
}

impl WithdrawRequest {
    /// Parse a WithdrawRequest from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();

        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let withdraw_account_id = args
            .get("withdrawAccountId")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'withdrawAccountId' field")?
            .to_string();

        let amount = args
            .get("amount")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'amount' field")?
            .to_string();

        let destination_btc_address = args
            .get("destinationBtcAddress")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'destinationBtcAddress' field")?
            .to_string();

        let btc_tx_id = args
            .get("btcTxId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Self {
            contract_id,
            withdraw_account_id,
            amount,
            destination_btc_address,
            btc_tx_id,
        })
    }
}

/// A CBTC token holding contract
#[derive(Debug, Clone)]
pub struct Holding {
    pub contract_id: String,
    pub amount: String,
    pub instrument_id: String,
    pub owner: String,
}

impl Holding {
    /// Parse a Holding from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();

        let args = contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .ok_or("createArgument is not an object")?;

        let amount = args
            .get("amount")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'amount' field")?
            .to_string();

        let instrument = args
            .get("instrument")
            .and_then(|v| v.as_object())
            .ok_or("Missing 'instrument' field")?;

        let instrument_id = instrument
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'instrument.id' field")?
            .to_string();

        let owner = args
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'owner' field")?
            .to_string();

        Ok(Self {
            contract_id,
            amount,
            instrument_id,
            owner,
        })
    }

    /// Check if this holding is locked (being used in another transaction)
    /// Returns true if the holding has a non-null lock field
    pub fn is_locked_in_contract(contract: &JsActiveContract) -> bool {
        contract
            .created_event
            .create_argument
            .as_ref()
            .and_then(|opt| opt.as_ref())
            .and_then(|v| v.as_object())
            .and_then(|args| args.get("lock"))
            .is_some_and(|lock| !lock.is_null())
    }
}
