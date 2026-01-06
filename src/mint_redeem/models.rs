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
    /// The account UUID from the contract's createArgument `id` field.
    /// This is the ID used by the attestor to look up the Bitcoin address.
    /// May be None for older accounts - in that case, use `contract_id` instead.
    pub id: Option<String>,
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

        // The `id` field is used by the attestor to look up the Bitcoin address.
        // May be null for older accounts.
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

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
            id,
            owner,
            operator,
            registrar,
            last_processed_bitcoin_block,
        })
    }

    /// Get the account ID used for attestor lookups (Bitcoin address, etc.)
    /// Uses the `id` field if present, otherwise falls back to `contract_id`.
    pub fn account_id(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.contract_id)
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
    pub template_id: String,
    pub owner: String,
    pub operator: String,
    pub registrar: String,
    pub destination_btc_address: String,
    pub pending_balance: String,
    pub created_event_blob: String,
}

impl WithdrawAccount {
    /// Parse a WithdrawAccount from a JsActiveContract
    pub fn from_active_contract(contract: &JsActiveContract) -> Result<Self, String> {
        let contract_id = contract.created_event.contract_id.clone();
        let template_id = contract.created_event.template_id.clone();
        let created_event_blob = contract.created_event.created_event_blob.clone();

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

        let pending_balance = args
            .get("pendingBalance")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0")
            .to_string();

        Ok(Self {
            contract_id,
            template_id,
            owner,
            operator,
            registrar,
            destination_btc_address,
            pending_balance,
            created_event_blob,
        })
    }
}

/// A withdraw request contract representing a CBTC burn and pending BTC withdrawal
///
/// Created by the registrar (attestor network) after the user submits a withdrawal.
/// The btc_tx_id contains the Bitcoin transaction ID used to fulfill the withdrawal.
#[derive(Debug, Clone)]
pub struct WithdrawRequest {
    pub contract_id: String,
    pub owner: String,
    pub registrar: String,
    pub amount: String,
    pub destination_btc_address: String,
    pub btc_tx_id: String,
    pub source_account_id: Option<String>,
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

        let owner = args
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'owner' field")?
            .to_string();

        let registrar = args
            .get("registrar")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'registrar' field")?
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
            .ok_or("Missing 'btcTxId' field")?
            .to_string();

        // sourceAccountId is Optional in Daml, so handle both Some and None cases
        let source_account_id = args
            .get("sourceAccountId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Self {
            contract_id,
            owner,
            registrar,
            amount,
            destination_btc_address,
            btc_tx_id,
            source_account_id,
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
