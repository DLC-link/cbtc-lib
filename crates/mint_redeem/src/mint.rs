use crate::attestor;
use crate::constants::{CREATE_DEPOSIT_ACCOUNT_CHOICE, DEPOSIT_ACCOUNT_TEMPLATE_ID, DEPOSIT_REQUEST_TEMPLATE_ID};
use crate::models::{AccountContractRuleSet, DepositAccount, DepositAccountStatus, DepositRequest};
use common::submission;
use common::transfer::DisclosedContract;
use ledger::active_contracts;
use ledger::common::{TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter};
use ledger::ledger_end;
use ledger::submit;
use serde_json::json;

/// Parameters for listing deposit accounts
pub struct ListDepositAccountsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for creating a deposit account
pub struct CreateDepositAccountParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub account_rules: AccountContractRuleSet,
}

/// Parameters for getting a deposit account's Bitcoin address
pub struct GetBitcoinAddressParams {
    pub attestor_url: String,
    pub account_contract_id: String,
    pub chain: String,
}

/// Parameters for listing deposit requests
pub struct ListDepositRequestsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for getting deposit account status
pub struct GetDepositAccountStatusParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    pub attestor_url: String,
    pub chain: String,
    pub account_contract_id: String,
}

/// List all deposit accounts for a party
///
/// # Example
/// ```ignore
/// let accounts = mint::list_deposit_accounts(ListDepositAccountsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
/// ```
pub async fn list_deposit_accounts(
    params: ListDepositAccountsParams,
) -> Result<Vec<DepositAccount>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for DepositAccount contracts
    let filter = ledger::common::IdentifierFilter::TemplateIdentifierFilter(
        TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(DEPOSIT_ACCOUNT_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        },
    );

    // Get active contracts
    let contracts = active_contracts::get_by_party(active_contracts::Params {
        ledger_host: params.ledger_host,
        party: params.party,
        filter,
        access_token: params.access_token,
        ledger_end: ledger_end_response.offset,
        unknown_contract_entry_handler: None,
    })
    .await?;

    let deposit_accounts: Result<Vec<DepositAccount>, String> = contracts
        .iter()
        .map(DepositAccount::from_active_contract)
        .collect();

    deposit_accounts
}

/// Create a new deposit account
///
/// This creates a DepositAccount contract on Canton that can receive BTC deposits.
///
/// # Example
/// ```ignore
/// use mint_redeem::attestor;
///
/// // First get the account rules from the attestor
/// let rules = attestor::get_account_contract_rules(
///     "https://devnet.dlc.link/attestor-1",
///     "canton-devnet"
/// ).await?;
///
/// // Create the deposit account
/// let account = mint::create_deposit_account(CreateDepositAccountParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     user_name: "user@example.com".to_string(),
///     access_token: "your-token".to_string(),
///     account_rules: rules,
/// }).await?;
/// ```
pub async fn create_deposit_account(
    params: CreateDepositAccountParams,
) -> Result<DepositAccount, String> {
    // Generate a random command ID
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    // Build the disclosed contracts - just the DepositAccountRules
    let disclosed_contracts = vec![DisclosedContract {
        contract_id: params.account_rules.da_rules.contract_id.clone(),
        created_event_blob: params.account_rules.da_rules.created_event_blob.clone(),
        template_id: params.account_rules.da_rules.template_id.clone(),
        synchronizer_id: String::new(),
    }];

    // Build the choice argument
    let choice_argument = json!({
        "owner": params.party
    });

    // Build the exercise command
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: params.account_rules.da_rules.template_id.clone(),
            contract_id: params.account_rules.da_rules.contract_id.clone(),
            choice: CREATE_DEPOSIT_ACCOUNT_CHOICE.to_string(),
            choice_argument: submission::ChoiceArgumentsVariations::Generic(choice_argument),
        },
    };

    // Build submission request
    let submission_request = submission::Submission {
        act_as: vec![params.party.clone()],
        command_id,
        disclosed_contracts,
        commands: vec![submission::Command::ExerciseCommand(exercise_command)],
        read_as: Some(vec![params.party.clone()]),
        user_id: Some(params.user_name.clone()),
    };

    // Submit the transaction
    let response_raw = submit::wait_for_transaction_tree(submit::Params {
        ledger_host: params.ledger_host.clone(),
        access_token: params.access_token.clone(),
        request: submission_request,
    })
    .await?;

    // Parse the response to extract the created DepositAccount
    let response: serde_json::Value = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    // Extract the created DepositAccount from eventsById
    let events_by_id = response["transactionTree"]["eventsById"]
        .as_object()
        .ok_or("Failed to find eventsById in transaction")?;

    for (_key, event) in events_by_id {
        if let Some(created_event) = event.get("CreatedTreeEvent") {
            let template_id = created_event["value"]["templateId"]
                .as_str()
                .unwrap_or("");

            // Match by suffix since template ID can be in different formats
            if template_id.ends_with(":CBTC.DepositAccount:CBTCDepositAccount") {
                // Parse the created event as a JsActiveContract
                let created_event_value = &created_event["value"];
                let active_contract = ledger::models::JsActiveContract {
                    created_event: Box::new(ledger::models::CreatedEvent {
                        contract_id: created_event_value["contractId"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        template_id: template_id.to_string(),
                        create_argument: Some(Some(created_event_value["createArgument"].clone())),
                        created_event_blob: created_event_value["createdEventBlob"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        ..Default::default()
                    }),
                    reassignment_counter: 0,
                    synchronizer_id: String::new(),
                };
                return DepositAccount::from_active_contract(&active_contract);
            }
        }
    }

    Err("No DepositAccount was created in the transaction".to_string())
}

/// Get the Bitcoin address for a deposit account
///
/// # Example
/// ```ignore
/// let bitcoin_address = mint::get_bitcoin_address(GetBitcoinAddressParams {
///     attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
///     account_contract_id: deposit_account.contract_id,
///     chain: "canton-devnet".to_string(),
/// }).await?;
///
/// println!("Send BTC to: {}", bitcoin_address);
/// ```
pub async fn get_bitcoin_address(params: GetBitcoinAddressParams) -> Result<String, String> {
    attestor::get_bitcoin_address(&params.attestor_url, &params.account_contract_id, &params.chain).await
}

/// List all deposit requests for a party
///
/// A deposit request is created after BTC is sent to a deposit account's address
/// and confirmed by the attestor network.
///
/// # Example
/// ```ignore
/// let requests = mint::list_deposit_requests(ListDepositRequestsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
///
/// for request in requests {
///     println!("Deposit: {} BTC in tx {}", request.amount, request.btc_tx_id);
/// }
/// ```
pub async fn list_deposit_requests(
    params: ListDepositRequestsParams,
) -> Result<Vec<DepositRequest>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for DepositRequest contracts
    let filter = ledger::common::IdentifierFilter::TemplateIdentifierFilter(
        TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(DEPOSIT_REQUEST_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        },
    );

    // Get active contracts
    let contracts = active_contracts::get_by_party(active_contracts::Params {
        ledger_host: params.ledger_host,
        party: params.party,
        filter,
        access_token: params.access_token,
        ledger_end: ledger_end_response.offset,
        unknown_contract_entry_handler: None,
    })
    .await?;

    let deposit_requests: Result<Vec<DepositRequest>, String> = contracts
        .iter()
        .map(DepositRequest::from_active_contract)
        .collect();

    deposit_requests
}

/// Get the full status of a deposit account including its Bitcoin address
///
/// # Example
/// ```ignore
/// let status = mint::get_deposit_account_status(GetDepositAccountStatusParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
///     attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
///     chain: "canton-devnet".to_string(),
///     account_contract_id: deposit_account.contract_id,
/// }).await?;
///
/// println!("Bitcoin address: {}", status.bitcoin_address);
/// ```
pub async fn get_deposit_account_status(
    params: GetDepositAccountStatusParams,
) -> Result<DepositAccountStatus, String> {
    // Get all deposit accounts
    let accounts = list_deposit_accounts(ListDepositAccountsParams {
        ledger_host: params.ledger_host,
        party: params.party,
        access_token: params.access_token,
    })
    .await?;

    // Find the account with matching contract ID
    let account = accounts
        .into_iter()
        .find(|a| a.contract_id == params.account_contract_id)
        .ok_or_else(|| format!("Deposit account with contract ID {} not found", params.account_contract_id))?;

    // Get the Bitcoin address from attestor
    let bitcoin_address =
        attestor::get_bitcoin_address(&params.attestor_url, &params.account_contract_id, &params.chain)
            .await?;

    Ok(DepositAccountStatus {
        contract_id: account.contract_id,
        owner: account.owner,
        operator: account.operator,
        registrar: account.registrar,
        bitcoin_address,
        last_processed_bitcoin_block: account.last_processed_bitcoin_block,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::login::{password, password_url, PasswordParams};
    use std::env;

    #[tokio::test]
    async fn test_list_deposit_accounts() {
        dotenvy::dotenv().ok();

        let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
        let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");

        let params = PasswordParams {
            client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set"),
            username: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            password: env::var("KEYCLOAK_PASSWORD").expect("KEYCLOAK_PASSWORD must be set"),
            url: password_url(
                &env::var("KEYCLOAK_HOST").expect("KEYCLOAK_HOST must be set"),
                &env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM must be set"),
            ),
        };
        let login_response = password(params).await.unwrap();

        let accounts = list_deposit_accounts(ListDepositAccountsParams {
            ledger_host,
            party: party_id,
            access_token: login_response.access_token,
        })
        .await
        .expect("Failed to list deposit accounts");

        assert!(!accounts.is_empty());
    }
}
