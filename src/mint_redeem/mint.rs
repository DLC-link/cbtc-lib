use crate::mint_redeem::attestor;
use crate::mint_redeem::constants::{
    CREATE_DEPOSIT_ACCOUNT_CHOICE, DEPOSIT_ACCOUNT_RULES_TEMPLATE_ID, DEPOSIT_ACCOUNT_TEMPLATE_ID,
};
use crate::mint_redeem::models::{AccountContractRuleSet, DepositAccount, DepositAccountStatus};
use common::submission;
use common::transfer::DisclosedContract;
use ledger::active_contracts;
use ledger::common::{TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter};
use ledger::ledger_end;
use ledger::models::{Event, JsSubmitAndWaitForTransactionResponse};
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
    pub credential_cids: Vec<String>,
}

/// Parameters for getting a deposit account's Bitcoin address
pub struct GetBitcoinAddressParams {
    pub api_url: String,
    pub account_id: String,
}

/// Parameters for getting deposit account status
pub struct GetDepositAccountStatusParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    pub api_url: String,
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
    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(DEPOSIT_ACCOUNT_TEMPLATE_ID.to_string()),
                    include_created_event_blob: true,
                },
            },
        });

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

/// Extract the contract ID of the newly created CBTCDepositAccount from a
/// flat-shaped submit response.
///
/// Walks `transaction.events`, finds the first `CreatedEvent` whose
/// `templateId` ends with `:CBTC.DepositAccount:CBTCDepositAccount` and
/// returns its `contractId`.
///
/// Accepts a typed `JsSubmitAndWaitForTransactionResponse` so that field-name
/// typos (e.g., `templateID` vs `templateId`) are caught at compile time.
fn parse_created_deposit_account_cid(
    response: &JsSubmitAndWaitForTransactionResponse,
) -> Result<String, String> {
    let events = response
        .transaction
        .events
        .as_ref()
        .ok_or("Failed to find events in transaction")?;

    for event in events {
        if let Event::EventOneOf1(wrapper) = event {
            let created = &wrapper.created_event;
            if created
                .template_id
                .ends_with(":CBTC.DepositAccount:CBTCDepositAccount")
            {
                return Ok(created.contract_id.clone());
            }
        }
    }

    Err("No DepositAccount was created in the transaction".to_string())
}

/// Create a new deposit account
///
/// This creates a DepositAccount contract on Canton that can receive BTC deposits.
///
/// # Example
/// ```ignore
/// use mint_redeem::attestor;
///
/// // First get the account rules from the Bitsafe API
/// let rules = attestor::get_account_contract_rules(
///     "https://api.mainnet.bitsafe.finance"
/// ).await?;
///
/// // Create the deposit account
/// let account = mint::create_deposit_account(CreateDepositAccountParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     user_name: "user@example.com".to_string(),
///     access_token: "your-token".to_string(),
///     account_rules: rules,
///     credential_cids: vec!["00abc...".to_string()],
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
        template_id: Some(params.account_rules.da_rules.template_id.clone()),
        synchronizer_id: String::new(),
    }];

    // Build the choice argument
    let choice_argument = json!({
        "owner": params.party,
        "credentialCids": params.credential_cids
    });

    // Build the exercise command
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: DEPOSIT_ACCOUNT_RULES_TEMPLATE_ID.to_string(),
            contract_id: params.account_rules.da_rules.contract_id.clone(),
            choice: CREATE_DEPOSIT_ACCOUNT_CHOICE.to_string(),
            choice_argument: submission::ChoiceArgumentsVariations::Generic(choice_argument),
        },
    };

    // Build submission request
    let submission_request = submission::Submission {
        act_as: vec![params.party.clone()],
        read_as: None,
        command_id,
        disclosed_contracts,
        commands: vec![submission::Command::ExerciseCommand(exercise_command)],
        ..Default::default()
    };

    // Submit the transaction
    let response_raw = submit::wait_for_transaction(submit::Params {
        ledger_host: params.ledger_host.clone(),
        access_token: params.access_token.clone(),
        request: submission_request,
    })
    .await?;

    // Parse the response to extract the contract ID of the created DepositAccount
    let response: JsSubmitAndWaitForTransactionResponse = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    let contract_id = parse_created_deposit_account_cid(&response)?;

    // Re-fetch from active contracts for the canonical DepositAccount shape.
    // (The flat submit response does include createArgument and createdEventBlob, so
    // this round-trip could be optimized away in a follow-up; see credentials.rs.)
    let accounts = list_deposit_accounts(ListDepositAccountsParams {
        ledger_host: params.ledger_host,
        party: params.party,
        access_token: params.access_token,
    })
    .await?;

    accounts
        .into_iter()
        .find(|a| a.contract_id == contract_id)
        .ok_or_else(|| {
            format!(
                "Created DepositAccount {} not found in active contracts",
                contract_id
            )
        })
}

/// Get the Bitcoin address for a deposit account
///
/// # Example
/// ```ignore
/// let bitcoin_address = mint::get_bitcoin_address(GetBitcoinAddressParams {
///     api_url: "https://api.mainnet.bitsafe.finance".to_string(),
///     account_id: deposit_account.contract_id,
/// }).await?;
///
/// log::debug!("Send BTC to: {}", bitcoin_address);
/// ```
pub async fn get_bitcoin_address(params: GetBitcoinAddressParams) -> Result<String, String> {
    attestor::get_bitcoin_address(&params.api_url, &params.account_id).await
}

/// Get the full status of a deposit account including its Bitcoin address
///
/// # Example
/// ```ignore
/// let status = mint::get_deposit_account_status(GetDepositAccountStatusParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
///     api_url: "https://api.mainnet.bitsafe.finance".to_string(),
///     account_contract_id: deposit_account.contract_id,
/// }).await?;
///
/// log::debug!("Bitcoin address: {}", status.bitcoin_address);
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
        .ok_or_else(|| {
            format!(
                "Deposit account with contract ID {} not found",
                params.account_contract_id
            )
        })?;

    // Get the Bitcoin address from Bitsafe API using the account's ID
    let bitcoin_address =
        attestor::get_bitcoin_address(&params.api_url, account.account_id()).await?;

    Ok(DepositAccountStatus {
        contract_id: account.contract_id,
        owner: account.owner,
        operator: account.operator,
        registrar: account.registrar,
        bitcoin_address,
        last_processed_bitcoin_block: account.last_processed_bitcoin_block,
        limits: account.limits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::login::{PasswordParams, password, password_url};
    use std::env;

    #[tokio::test]
    async fn test_create_deposit_account_with_credentials() {
        dotenvy::dotenv().ok();

        let ledger_host = env::var("LEDGER_HOST").expect("LEDGER_HOST must be set");
        let party_id = env::var("PARTY_ID").expect("PARTY_ID must be set");
        let api_url = env::var("BITSAFE_API_URL").expect("BITSAFE_API_URL must be set");

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
        let access_token = login_response.access_token;

        // Fetch credentials
        let credentials =
            crate::credentials::list_credentials(crate::credentials::ListCredentialsParams {
                ledger_host: ledger_host.clone(),
                party: party_id.clone(),
                access_token: access_token.clone(),
            })
            .await
            .expect("Failed to list credentials");

        let minter_credential_cids: Vec<String> = credentials
            .iter()
            .filter(|c| {
                c.claims
                    .iter()
                    .any(|claim| claim.property == "hasCBTCRole" && claim.value == "Minter")
            })
            .map(|c| c.contract_id.clone())
            .collect();

        assert!(
            !minter_credential_cids.is_empty(),
            "No Minter credentials found for party"
        );

        // Fetch account rules
        let account_rules = crate::mint_redeem::attestor::get_account_contract_rules(&api_url)
            .await
            .expect("Failed to get account rules");

        // Create deposit account with credentials
        let account = create_deposit_account(CreateDepositAccountParams {
            ledger_host,
            party: party_id.clone(),
            user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            access_token,
            account_rules,
            credential_cids: minter_credential_cids,
        })
        .await
        .expect("Failed to create deposit account with credentials");

        assert_eq!(account.owner, party_id);
        assert!(!account.contract_id.is_empty());
    }

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

#[cfg(test)]
mod parser_tests {
    //! Pure-data fixture tests for the flat-event parser used by
    //! `create_deposit_account`. These tests do not touch the network and
    //! exercise only the typed-event matching logic in
    //! `parse_created_deposit_account_cid`.

    use super::*;
    use crate::utils::test_fixtures::{
        created_event_value, exercised_event_value, transaction_response,
    };
    use serde_json::json;

    #[test]
    fn parses_contract_id_from_created_deposit_account_event() {
        let response = transaction_response(
            "tx-1",
            json!([
                exercised_event_value(
                    "pkg:CBTC.DepositAccount:CBTCDepositAccountRules",
                    "CBTCDepositAccountRules_CreateDepositAccount",
                    json!(null),
                ),
                created_event_value(
                    "f240dd5d1a98079f37c0f93272cf5b28d4523027c42d0003c4c7a530eed6c313:CBTC.DepositAccount:CBTCDepositAccount",
                    "000b5aff71065dc7f8be2b72991574e8ec5382ec0672eaa52bf0022ed97bdd94f5",
                    json!(null),
                ),
            ]),
        );

        let result = parse_created_deposit_account_cid(&response);
        assert_eq!(
            result.unwrap(),
            "000b5aff71065dc7f8be2b72991574e8ec5382ec0672eaa52bf0022ed97bdd94f5"
        );
    }

    #[test]
    fn returns_err_when_no_matching_template() {
        // Events present but template suffix doesn't match — e.g., only the
        // rules-side ExercisedEvent and an unrelated CreatedEvent.
        let response = transaction_response(
            "tx-2",
            json!([
                exercised_event_value(
                    "pkg:CBTC.DepositAccount:CBTCDepositAccountRules",
                    "CBTCDepositAccountRules_CreateDepositAccount",
                    json!(null),
                ),
                created_event_value("pkg:Some.Other:Template", "00other", json!(null)),
            ]),
        );

        let err = parse_created_deposit_account_cid(&response).unwrap_err();
        assert!(
            err.contains("No DepositAccount was created"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn returns_err_when_events_missing() {
        // Envelope is missing `transaction.events` entirely.
        let response = transaction_response("tx-3", json!(null));

        let err = parse_created_deposit_account_cid(&response).unwrap_err();
        assert!(
            err.contains("Failed to find events"),
            "unexpected error message: {err}"
        );
    }
}
