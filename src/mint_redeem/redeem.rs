use crate::mint_redeem::attestor;
use crate::mint_redeem::constants::{
    CREATE_WITHDRAW_ACCOUNT_CHOICE, HOLDING_TEMPLATE_ID, WITHDRAW_ACCOUNT_RULES_TEMPLATE_ID,
    WITHDRAW_ACCOUNT_TEMPLATE_ID, WITHDRAW_CHOICE, WITHDRAW_REQUEST_TEMPLATE_ID,
};
use crate::mint_redeem::models::{
    Holding, TokenStandardContracts, WithdrawAccount, WithdrawRequest,
};
use common::submission;
use common::transfer::DisclosedContract;
use ledger::active_contracts;
use ledger::common::{TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter};
use ledger::ledger_end;
use ledger::models::{JsActiveContract, JsSubmitAndWaitForTransactionResponse};
use ledger::submit;
use serde_json::json;

/// Parameters for listing withdraw accounts
pub struct ListWithdrawAccountsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for creating a withdraw account
pub struct CreateWithdrawAccountParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub account_rules_contract_id: String,
    pub account_rules_template_id: String,
    pub account_rules_created_event_blob: String,
    pub destination_btc_address: String,
    pub credential_cids: Vec<String>,
}

/// Parameters for listing CBTC holdings
pub struct ListHoldingsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for submitting a withdrawal (burning CBTC)
///
/// After submitting, the user's tokens are burned and the pending_balance
/// on their WithdrawAccount is increased. The attestor network will later
/// create a WithdrawRequest to process the BTC payout.
pub struct SubmitWithdrawParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub api_url: String,
    pub withdraw_account_contract_id: String,
    pub withdraw_account_template_id: String,
    pub amount: common::decimal::DamlDecimal,
    pub holding_contract_ids: Vec<String>,
    pub credential_cids: Option<Vec<String>>,
}

/// Parameters for listing withdraw requests
pub struct ListWithdrawRequestsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// List all withdraw accounts for a party
///
/// # Example
/// ```ignore
/// let accounts = redeem::list_withdraw_accounts(ListWithdrawAccountsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
/// ```
pub async fn list_withdraw_accounts(
    params: ListWithdrawAccountsParams,
) -> Result<Vec<WithdrawAccount>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for WithdrawAccount contracts
    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(WITHDRAW_ACCOUNT_TEMPLATE_ID.to_string()),
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

    let withdraw_accounts: Result<Vec<WithdrawAccount>, String> = contracts
        .iter()
        .map(WithdrawAccount::from_active_contract)
        .collect();

    withdraw_accounts
}

/// Extract the contract ID of the newly created CBTCWithdrawAccount from a
/// flat-shaped submit response.
///
/// Walks `transaction.events`, finds the first `CreatedEvent` whose
/// `templateId` ends with `:CBTC.WithdrawAccount:CBTCWithdrawAccount` and
/// returns its `contractId`.
///
/// Accepts a typed `JsSubmitAndWaitForTransactionResponse` so that field-name
/// typos are caught at compile time.
fn parse_created_withdraw_account_cid(
    response: &JsSubmitAndWaitForTransactionResponse,
) -> Result<String, String> {
    let events = &response.transaction.events;

    for event in events {
        if let Some(created) = crate::event_helpers::as_created_event(event) {
            if created
                .template_id
                .ends_with(":CBTC.WithdrawAccount:CBTCWithdrawAccount")
            {
                return Ok(created.contract_id.clone());
            }
        }
    }

    Err("No WithdrawAccount was created in the transaction".to_string())
}

/// Extract the updated WithdrawAccount from a flat-shaped submit response for
/// the Withdraw choice. The Withdraw choice consumes the old account and
/// creates a new one with an updated `pendingBalance`.
fn parse_submit_withdraw_response(
    response: &JsSubmitAndWaitForTransactionResponse,
) -> Result<WithdrawAccount, String> {
    let events = &response.transaction.events;

    for event in events {
        if let Some(created) = crate::event_helpers::as_created_event(event) {
            // Match by suffix since template ID can be in different formats
            if created
                .template_id
                .ends_with(":CBTC.WithdrawAccount:CBTCWithdrawAccount")
            {
                let active_contract = JsActiveContract {
                    created_event: Box::new(created.clone()),
                    reassignment_counter: 0,
                    synchronizer_id: String::new(),
                };
                return WithdrawAccount::from_active_contract(&active_contract);
            }
        }
    }

    Err("No updated WithdrawAccount was found in the transaction".to_string())
}

/// Create a new withdraw account
///
/// This creates a WithdrawAccount contract on Canton that can be used to burn CBTC
/// and receive BTC at the specified destination address.
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
/// // Create the withdraw account with a BTC address
/// let account = redeem::create_withdraw_account(CreateWithdrawAccountParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     user_name: "user@example.com".to_string(),
///     access_token: "your-token".to_string(),
///     account_rules_contract_id: rules.wa_rules.contract_id,
///     account_rules_template_id: rules.wa_rules.template_id,
///     account_rules_created_event_blob: rules.wa_rules.created_event_blob,
///     destination_btc_address: "bc1q...".to_string(),
///     credential_cids: vec!["00abc...".to_string()],
/// }).await?;
/// ```
pub async fn create_withdraw_account(
    params: CreateWithdrawAccountParams,
) -> Result<WithdrawAccount, String> {
    // Generate a random command ID
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    // Build the disclosed contracts - just the WithdrawAccountRules
    let disclosed_contracts = vec![DisclosedContract {
        contract_id: params.account_rules_contract_id.clone(),
        created_event_blob: params.account_rules_created_event_blob.clone(),
        template_id: Some(params.account_rules_template_id.clone()),
        synchronizer_id: String::new(),
    }];

    // Build the choice argument
    let choice_argument = json!({
        "owner": params.party,
        "destinationBtcAddress": params.destination_btc_address,
        "credentialCids": params.credential_cids
    });

    // Build the exercise command
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: WITHDRAW_ACCOUNT_RULES_TEMPLATE_ID.to_string(),
            contract_id: params.account_rules_contract_id.clone(),
            choice: CREATE_WITHDRAW_ACCOUNT_CHOICE.to_string(),
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

    // Parse the response to extract the contract ID of the created WithdrawAccount
    let response: JsSubmitAndWaitForTransactionResponse = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    let contract_id = parse_created_withdraw_account_cid(&response)?;

    // Re-fetch from active contracts for the canonical WithdrawAccount shape.
    // (The flat submit response does include createArgument and createdEventBlob, so
    // this round-trip could be optimized away in a follow-up; see credentials.rs.)
    let accounts = list_withdraw_accounts(ListWithdrawAccountsParams {
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
                "Created WithdrawAccount {} not found in active contracts",
                contract_id
            )
        })
}

/// List all CBTC holdings (token contracts) for a party
///
/// # Example
/// ```ignore
/// let holdings = redeem::list_holdings(ListHoldingsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
///
/// let total_cbtc: common::decimal::DamlDecimal = holdings.iter()
///     .filter(|h| h.instrument_id == "CBTC")
///     .map(|h| h.amount)
///     .sum();
/// log::debug!("Total CBTC holdings: {}", total_cbtc);
/// ```
pub async fn list_holdings(params: ListHoldingsParams) -> Result<Vec<Holding>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for Holding contracts
    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(HOLDING_TEMPLATE_ID.to_string()),
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

    // Filter out locked holdings (those being used in other transactions)
    // and parse the remaining ones
    let holdings: Result<Vec<Holding>, String> = contracts
        .iter()
        .filter(|contract| !Holding::is_locked_in_contract(contract))
        .map(Holding::from_active_contract)
        .collect();

    holdings
}

/// Submit a withdrawal by burning CBTC holdings
///
/// This burns the specified CBTC holdings and increases the pending_balance on
/// the user's WithdrawAccount. The attestor network will later create a
/// WithdrawRequest to process the BTC payout to the destination address.
///
/// Note: WithdrawRequests are NOT created atomically with this call. Use
/// `list_withdraw_requests()` to periodically check for processed withdrawals.
///
/// # Example
/// ```ignore
/// // First get your holdings
/// let holdings = redeem::list_holdings(ListHoldingsParams {
///     ledger_host: ledger_host.clone(),
///     party: party_id.clone(),
///     access_token: access_token.clone(),
/// }).await?;
///
/// // Select holdings to burn (must have enough CBTC)
/// let holding_ids: Vec<String> = holdings.iter()
///     .filter(|h| h.instrument_id == "CBTC")
///     .take(1) // Simplest case: use one holding
///     .map(|h| h.contract_id.clone())
///     .collect();
///
/// // Submit withdrawal - burns tokens and increases pending_balance
/// let updated_account = redeem::submit_withdraw(SubmitWithdrawParams {
///     ledger_host: ledger_host.clone(),
///     party: party_id.clone(),
///     access_token: access_token.clone(),
///     api_url: "https://api.mainnet.bitsafe.finance".to_string(),
///     withdraw_account_contract_id: withdraw_account.contract_id,
///     withdraw_account_template_id: withdraw_account.template_id,
///     amount: common::decimal::DamlDecimal::parse("0.001").unwrap(),
///     holding_contract_ids: holding_ids,
/// }).await?;
///
/// println!("Pending balance: {}", updated_account.pending_balance);
/// // Later, check for WithdrawRequests using list_withdraw_requests()
/// ```
pub async fn submit_withdraw(params: SubmitWithdrawParams) -> Result<WithdrawAccount, String> {
    // Get token standard contracts from Bitsafe API
    let token_contracts: TokenStandardContracts =
        attestor::get_token_standard_contracts(&params.api_url).await?;

    // Generate a random command ID
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    // Build disclosed contracts - include withdraw account and all token standard contracts
    let disclosed_contracts = vec![
        // Withdraw account being exercised
        DisclosedContract {
            contract_id: token_contracts.burn_mint_factory.contract_id.clone(),
            created_event_blob: token_contracts.burn_mint_factory.created_event_blob.clone(),
            template_id: Some(token_contracts.burn_mint_factory.template_id.clone()),
            synchronizer_id: String::new(),
        },
        DisclosedContract {
            contract_id: token_contracts.instrument_configuration.contract_id.clone(),
            created_event_blob: token_contracts
                .instrument_configuration
                .created_event_blob
                .clone(),
            template_id: Some(token_contracts.instrument_configuration.template_id.clone()),
            synchronizer_id: String::new(),
        },
        DisclosedContract {
            contract_id: token_contracts.issuer_credential.contract_id.clone(),
            created_event_blob: token_contracts.issuer_credential.created_event_blob.clone(),
            template_id: Some(token_contracts.issuer_credential.template_id.clone()),
            synchronizer_id: String::new(),
        },
    ];

    // Build extraArgs for burn operation with proper token standard context structure
    let mut context_values = serde_json::Map::new();

    // Add instrument configuration
    context_values.insert(
        "utility.digitalasset.com/instrument-configuration".to_string(),
        json!({
            "tag": "AV_ContractId",
            "value": token_contracts.instrument_configuration.contract_id
        }),
    );

    // Add issuer credentials as a list
    context_values.insert(
        "utility.digitalasset.com/issuer-credentials".to_string(),
        json!({
            "tag": "AV_List",
            "value": [{
                "tag": "AV_ContractId",
                "value": token_contracts.issuer_credential.contract_id
            }]
        }),
    );

    let extra_args = json!({
        "context": {
            "values": context_values
        },
        "meta": {
            "values": {
                "splice.lfdecentralizedtrust.org/reason": "CBTC withdrawal"
            }
        }
    });

    // Build choice argument JSON manually to preserve decimal format
    // serde_json can use scientific notation for small numbers, which Canton rejects
    // Keep amount as a JSON string (quoted) to ensure Canton receives it in decimal format
    let credential_cids_json = match &params.credential_cids {
        Some(cids) => serde_json::to_string(cids).unwrap(),
        None => "null".to_string(),
    };

    let choice_argument_str = format!(
        r#"{{
            "tokens": {},
            "amount": "{}",
            "burnMintFactoryCid": "{}",
            "extraArgs": {},
            "credentialCids": {}
        }}"#,
        serde_json::to_string(&params.holding_contract_ids).unwrap(),
        params.amount, // Keep as quoted string
        token_contracts.burn_mint_factory.contract_id,
        serde_json::to_string(&extra_args).unwrap(),
        credential_cids_json
    );

    let choice_argument: serde_json::Value = serde_json::from_str(&choice_argument_str)
        .map_err(|e| format!("Failed to construct choice argument: {}", e))?;

    // Build the exercise command
    // Use the actual template_id from the contract, not the #cbtc shorthand
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: params.withdraw_account_template_id.clone(),
            contract_id: params.withdraw_account_contract_id.clone(),
            choice: WITHDRAW_CHOICE.to_string(),
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

    // Parse the response to extract the updated WithdrawAccount
    let response: JsSubmitAndWaitForTransactionResponse = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    parse_submit_withdraw_response(&response)
}

/// List all withdraw requests for a party
///
/// Withdraw requests are created by the attestor network (registrar) after a user
/// submits a withdrawal via `submit_withdraw()`. The creation of WithdrawRequests
/// is NOT atomic with the withdrawal submission - it happens later when the
/// attestor processes the pending balance.
///
/// Each WithdrawRequest includes a `btc_tx_id` which is the Bitcoin transaction
/// ID used to fulfill the withdrawal.
///
/// # Example
/// ```ignore
/// // Periodically poll for withdraw requests
/// let requests = redeem::list_withdraw_requests(ListWithdrawRequestsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
///
/// for request in requests {
///     log::debug!(
///         "Withdrawal: {} BTC to {} (tx: {})",
///         request.amount,
///         request.destination_btc_address,
///         request.btc_tx_id
///     );
/// }
/// ```
pub async fn list_withdraw_requests(
    params: ListWithdrawRequestsParams,
) -> Result<Vec<WithdrawRequest>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for WithdrawRequest contracts
    let filter =
        ledger::common::IdentifierFilter::TemplateIdentifierFilter(TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(WITHDRAW_REQUEST_TEMPLATE_ID.to_string()),
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

    let withdraw_requests: Result<Vec<WithdrawRequest>, String> = contracts
        .iter()
        .map(WithdrawRequest::from_active_contract)
        .collect();

    withdraw_requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::login::{PasswordParams, password, password_url};
    use std::env;

    #[tokio::test]
    async fn test_create_withdraw_account_with_credentials() {
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

        // Create withdraw account with credentials
        let account = create_withdraw_account(CreateWithdrawAccountParams {
            ledger_host,
            party: party_id.clone(),
            user_name: env::var("KEYCLOAK_USERNAME").expect("KEYCLOAK_USERNAME must be set"),
            access_token,
            account_rules_contract_id: account_rules.wa_rules.contract_id,
            account_rules_template_id: account_rules.wa_rules.template_id,
            account_rules_created_event_blob: account_rules.wa_rules.created_event_blob,
            destination_btc_address: "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
            credential_cids: minter_credential_cids,
        })
        .await
        .expect("Failed to create withdraw account with credentials");

        assert_eq!(account.owner, party_id);
        assert!(!account.contract_id.is_empty());
    }

    #[tokio::test]
    async fn test_list_withdraw_accounts() {
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

        let accounts = list_withdraw_accounts(ListWithdrawAccountsParams {
            ledger_host,
            party: party_id,
            access_token: login_response.access_token,
        })
        .await
        .expect("Failed to list withdraw accounts");

        assert!(!accounts.is_empty());
    }
}

#[cfg(test)]
mod parser_tests {
    //! Pure-data fixture tests for the flat-event parsers used by
    //! `create_withdraw_account` and `submit_withdraw`. These tests do not
    //! touch the network and exercise only the typed-event matching logic in
    //! `parse_created_withdraw_account_cid` and
    //! `parse_submit_withdraw_response`.

    use super::*;
    use crate::utils::test_fixtures::{
        created_event_value, created_event_value_with_blob, exercised_event_value,
        transaction_response,
    };
    use serde_json::json;

    const WITHDRAW_ACCOUNT_TID: &str =
        "pkg-hash:CBTC.WithdrawAccount:CBTCWithdrawAccount";

    fn withdraw_account_create_argument() -> serde_json::Value {
        // Mirrors the shape of `createArgument` produced by the JSON ledger
        // API for a CBTCWithdrawAccount contract — the fields used by
        // `WithdrawAccount::from_active_contract`.
        json!({
            "owner": "alice::1220deadbeef",
            "operator": "operator::1220ababab",
            "registrar": "registrar::1220cdcdcd",
            "destinationBtcAddress": "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
            "pendingBalance": "0.0",
            "limits": null
        })
    }

    // ---------- parse_created_withdraw_account_cid ----------

    #[test]
    fn parse_created_withdraw_account_cid_happy_path() {
        let response = transaction_response(
            "tx-1",
            json!([
                exercised_event_value(
                    "pkg:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
                    "CBTCWithdrawAccountRules_CreateWithdrawAccount",
                    json!(null),
                ),
                created_event_value(WITHDRAW_ACCOUNT_TID, "00wac1", json!(null)),
            ]),
        );

        assert_eq!(
            parse_created_withdraw_account_cid(&response).unwrap(),
            "00wac1"
        );
    }

    #[test]
    fn parse_created_withdraw_account_cid_missing_match() {
        let response = transaction_response(
            "tx-x",
            json!([
                created_event_value("pkg:Some.Other:Template", "00other", json!(null)),
            ]),
        );

        let err = parse_created_withdraw_account_cid(&response).unwrap_err();
        assert!(
            err.contains("No WithdrawAccount was created"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_created_withdraw_account_cid_missing_events() {
        // `events` is required on the wire now; pass an empty list and verify
        // the parser falls through to its post-loop check.
        let response = transaction_response("tx-x", json!(null));
        let err = parse_created_withdraw_account_cid(&response).unwrap_err();
        assert!(
            err.contains("No WithdrawAccount was created"),
            "unexpected error: {err}"
        );
    }

    // ---------- parse_submit_withdraw_response ----------

    #[test]
    fn parse_submit_withdraw_response_happy_path() {
        let response = transaction_response(
            "tx-w1",
            json!([
                exercised_event_value(
                    WITHDRAW_ACCOUNT_TID,
                    "CBTCWithdrawAccount_Withdraw",
                    json!(null),
                ),
                created_event_value_with_blob(
                    WITHDRAW_ACCOUNT_TID,
                    "00new-withdraw-account",
                    withdraw_account_create_argument(),
                    "blob-base64",
                ),
            ]),
        );

        let account = parse_submit_withdraw_response(&response).unwrap();
        assert_eq!(account.contract_id, "00new-withdraw-account");
        assert_eq!(account.owner, "alice::1220deadbeef");
        assert_eq!(account.template_id, WITHDRAW_ACCOUNT_TID);
        assert_eq!(account.created_event_blob, "blob-base64");
        assert_eq!(
            account.destination_btc_address,
            "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"
        );
    }

    #[test]
    fn parse_submit_withdraw_response_missing_match() {
        // Only an unrelated CreatedEvent — no CBTCWithdrawAccount.
        let response = transaction_response(
            "tx-x",
            json!([
                created_event_value("pkg:Some.Other:Template", "00other", json!({})),
            ]),
        );

        let err = parse_submit_withdraw_response(&response).unwrap_err();
        assert!(
            err.contains("No updated WithdrawAccount was found"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_submit_withdraw_response_missing_events() {
        // `events` is required on the wire now; pass an empty list and verify
        // the parser falls through to its post-loop check.
        let response = transaction_response("tx-x", json!(null));
        let err = parse_submit_withdraw_response(&response).unwrap_err();
        assert!(
            err.contains("No updated WithdrawAccount was found"),
            "unexpected error: {err}"
        );
    }
}
