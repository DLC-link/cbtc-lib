use crate::attestor;
use crate::constants::{CREATE_WITHDRAW_ACCOUNT_CHOICE, HOLDING_TEMPLATE_ID, WITHDRAW_ACCOUNT_TEMPLATE_ID, WITHDRAW_CHOICE, WITHDRAW_REQUEST_TEMPLATE_ID};
use crate::models::{Holding, TokenStandardContracts, WithdrawAccount, WithdrawRequest};
use base64::Engine;
use common::submission;
use common::transfer::DisclosedContract;
use ledger::active_contracts;
use ledger::common::{TemplateFilter, TemplateFilterValue, TemplateIdentifierFilter};
use ledger::ledger_end;
use ledger::submit;
use serde_json::json;

/// Extract the user ID (subject claim) from a JWT access token
fn extract_user_id_from_jwt(access_token: &str) -> Result<String, String> {
    // JWT format: header.payload.signature
    let parts: Vec<&str> = access_token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }

    // Decode the payload (second part)
    let payload = parts[1];

    // URL-safe base64 without padding - we need to add padding for the decoder
    let padding_needed = (4 - (payload.len() % 4)) % 4;
    let padded = if padding_needed > 0 {
        format!("{}{}", payload, "=".repeat(padding_needed))
    } else {
        payload.to_string()
    };

    // Decode base64 - use STANDARD engine with padding since we added it
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&padded)
        .map_err(|e| format!("Failed to decode JWT payload: {}", e))?;

    // Parse JSON
    let json: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|e| format!("Failed to parse JWT payload JSON: {}", e))?;

    // Extract 'sub' claim (user ID / UUID)
    json.get("sub")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "JWT does not contain 'sub' claim".to_string())
}

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
}

/// Parameters for listing CBTC holdings
pub struct ListHoldingsParams {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
}

/// Parameters for requesting a withdrawal (burning CBTC)
pub struct RequestWithdrawParams {
    pub ledger_host: String,
    pub party: String,
    pub user_name: String,
    pub access_token: String,
    pub attestor_url: String,
    pub chain: String,
    pub withdraw_account_contract_id: String,
    pub amount: String,
    pub holding_contract_ids: Vec<String>,
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
    let filter = ledger::common::IdentifierFilter::TemplateIdentifierFilter(
        TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(WITHDRAW_ACCOUNT_TEMPLATE_ID.to_string()),
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

    let withdraw_accounts: Result<Vec<WithdrawAccount>, String> = contracts
        .iter()
        .map(WithdrawAccount::from_active_contract)
        .collect();

    withdraw_accounts
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
/// // First get the account rules from the attestor
/// let rules = attestor::get_account_contract_rules(
///     "https://devnet.dlc.link/attestor-1",
///     "canton-devnet"
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
/// }).await?;
/// ```
pub async fn create_withdraw_account(
    params: CreateWithdrawAccountParams,
) -> Result<WithdrawAccount, String> {
    // Extract user ID from JWT access token
    let user_id = extract_user_id_from_jwt(&params.access_token)?;

    // Generate a random command ID
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    // Build the disclosed contracts - just the WithdrawAccountRules
    let disclosed_contracts = vec![DisclosedContract {
        contract_id: params.account_rules_contract_id.clone(),
        created_event_blob: params.account_rules_created_event_blob.clone(),
        template_id: params.account_rules_template_id.clone(),
        synchronizer_id: String::new(),
    }];

    // Build the choice argument
    let choice_argument = json!({
        "owner": params.party,
        "destinationBtcAddress": params.destination_btc_address
    });

    // Build the exercise command
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: params.account_rules_template_id.clone(),
            contract_id: params.account_rules_contract_id.clone(),
            choice: CREATE_WITHDRAW_ACCOUNT_CHOICE.to_string(),
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
        user_id: Some(user_id),
    };

    // Submit the transaction
    let response_raw = submit::wait_for_transaction_tree(submit::Params {
        ledger_host: params.ledger_host.clone(),
        access_token: params.access_token.clone(),
        request: submission_request,
    })
    .await?;

    // Parse the response to extract the created WithdrawAccount
    let response: serde_json::Value = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    // Extract the created WithdrawAccount from eventsById
    let events_by_id = response["transactionTree"]["eventsById"]
        .as_object()
        .ok_or("Failed to find eventsById in transaction")?;

    for (_key, event) in events_by_id {
        if let Some(created_event) = event.get("CreatedTreeEvent") {
            let template_id = created_event["value"]["templateId"]
                .as_str()
                .unwrap_or("");

            // Match by suffix since template ID can be in different formats
            if template_id.ends_with(":CBTC.WithdrawAccount:CBTCWithdrawAccount") {
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
                return WithdrawAccount::from_active_contract(&active_contract);
            }
        }
    }

    Err("No WithdrawAccount was created in the transaction".to_string())
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
/// let total_cbtc: f64 = holdings.iter()
///     .filter(|h| h.instrument_id == "CBTC")
///     .map(|h| h.amount.parse::<f64>().unwrap_or(0.0))
///     .sum();
/// println!("Total CBTC holdings: {}", total_cbtc);
/// ```
pub async fn list_holdings(params: ListHoldingsParams) -> Result<Vec<Holding>, String> {
    // Get ledger end offset
    let ledger_end_response = ledger_end::get(ledger_end::Params {
        access_token: params.access_token.clone(),
        ledger_host: params.ledger_host.clone(),
    })
    .await?;

    // Create template filter for Holding contracts
    let filter = ledger::common::IdentifierFilter::TemplateIdentifierFilter(
        TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(HOLDING_TEMPLATE_ID.to_string()),
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

    // Filter out locked holdings (those being used in other transactions)
    // and parse the remaining ones
    let holdings: Result<Vec<Holding>, String> = contracts
        .iter()
        .filter(|contract| !Holding::is_locked_in_contract(contract))
        .map(Holding::from_active_contract)
        .collect();

    holdings
}

/// Request a withdrawal by burning CBTC holdings
///
/// This burns the specified CBTC holdings and creates a WithdrawRequest that will
/// be processed by the attestor network to send BTC to the withdraw account's destination address.
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
/// // Request withdrawal
/// let withdraw_request = redeem::request_withdraw(RequestWithdrawParams {
///     ledger_host: ledger_host.clone(),
///     party: party_id.clone(),
///     user_name: "user@example.com".to_string(),
///     access_token: access_token.clone(),
///     attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
///     chain: "canton-devnet".to_string(),
///     withdraw_account_contract_id: withdraw_account.contract_id,
///     amount: "0.001".to_string(),
///     holding_contract_ids: holding_ids,
/// }).await?;
/// ```
pub async fn request_withdraw(params: RequestWithdrawParams) -> Result<WithdrawRequest, String> {
    // Extract user ID from JWT access token
    let user_id = extract_user_id_from_jwt(&params.access_token)?;

    // Get token standard contracts from attestor
    let token_contracts: TokenStandardContracts =
        attestor::get_token_standard_contracts(&params.attestor_url, &params.chain).await?;

    // Generate a random command ID
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());

    // Build disclosed contracts - include all token standard contracts
    let mut disclosed_contracts = vec![
        DisclosedContract {
            contract_id: token_contracts.burn_mint_factory.contract_id.clone(),
            created_event_blob: token_contracts.burn_mint_factory.created_event_blob.clone(),
            template_id: token_contracts.burn_mint_factory.template_id.clone(),
            synchronizer_id: String::new(),
        },
        DisclosedContract {
            contract_id: token_contracts.instrument_configuration.contract_id.clone(),
            created_event_blob: token_contracts
                .instrument_configuration
                .created_event_blob
                .clone(),
            template_id: token_contracts.instrument_configuration.template_id.clone(),
            synchronizer_id: String::new(),
        },
    ];

    // Add optional contracts if present
    if let Some(issuer_credential) = &token_contracts.issuer_credential {
        disclosed_contracts.push(DisclosedContract {
            contract_id: issuer_credential.contract_id.clone(),
            created_event_blob: issuer_credential.created_event_blob.clone(),
            template_id: issuer_credential.template_id.clone(),
            synchronizer_id: String::new(),
        });
    }

    if let Some(app_reward_config) = &token_contracts.app_reward_configuration {
        disclosed_contracts.push(DisclosedContract {
            contract_id: app_reward_config.contract_id.clone(),
            created_event_blob: app_reward_config.created_event_blob.clone(),
            template_id: app_reward_config.template_id.clone(),
            synchronizer_id: String::new(),
        });
    }

    if let Some(featured_app_right) = &token_contracts.featured_app_right {
        disclosed_contracts.push(DisclosedContract {
            contract_id: featured_app_right.contract_id.clone(),
            created_event_blob: featured_app_right.created_event_blob.clone(),
            template_id: featured_app_right.template_id.clone(),
            synchronizer_id: String::new(),
        });
    }

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

    // Add issuer credentials as a list if present
    if let Some(issuer_cred) = &token_contracts.issuer_credential {
        context_values.insert(
            "utility.digitalasset.com/issuer-credentials".to_string(),
            json!({
                "tag": "AV_List",
                "value": [{
                    "tag": "AV_ContractId",
                    "value": issuer_cred.contract_id
                }]
            }),
        );
    }

    // Add app reward configuration if present
    if let Some(app_reward) = &token_contracts.app_reward_configuration {
        context_values.insert(
            "utility.digitalasset.com/app-reward-configuration".to_string(),
            json!({
                "tag": "AV_ContractId",
                "value": app_reward.contract_id
            }),
        );
    }

    // Add featured app right if present
    if let Some(featured_app) = &token_contracts.featured_app_right {
        context_values.insert(
            "utility.digitalasset.com/featured-app-right".to_string(),
            json!({
                "tag": "AV_ContractId",
                "value": featured_app.contract_id
            }),
        );
    }

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

    // Validate amount is a valid number
    let _: f64 = params.amount.parse()
        .map_err(|e| format!("Invalid amount format: {}", e))?;

    // Build choice argument JSON manually to preserve decimal format
    // serde_json can use scientific notation for small numbers, which Canton rejects
    // Keep amount as a JSON string (quoted) to ensure Canton receives it in decimal format
    let choice_argument_str = format!(
        r#"{{
            "tokens": {},
            "amount": "{}",
            "burnMintFactoryCid": "{}",
            "extraArgs": {}
        }}"#,
        serde_json::to_string(&params.holding_contract_ids).unwrap(),
        params.amount,  // Keep as quoted string
        token_contracts.burn_mint_factory.contract_id,
        serde_json::to_string(&extra_args).unwrap()
    );

    let choice_argument: serde_json::Value = serde_json::from_str(&choice_argument_str)
        .map_err(|e| format!("Failed to construct choice argument: {}", e))?;

    // Build the exercise command
    let exercise_command = submission::ExerciseCommand {
        exercise_command: submission::ExerciseCommandData {
            template_id: WITHDRAW_ACCOUNT_TEMPLATE_ID.to_string(),
            contract_id: params.withdraw_account_contract_id.clone(),
            choice: WITHDRAW_CHOICE.to_string(),
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
        user_id: Some(user_id),
    };

    // Submit the transaction
    let response_raw = submit::wait_for_transaction_tree(submit::Params {
        ledger_host: params.ledger_host.clone(),
        access_token: params.access_token.clone(),
        request: submission_request,
    })
    .await?;

    // Parse the response to extract the created WithdrawRequest
    let response: serde_json::Value = serde_json::from_str(&response_raw)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    // Extract the created WithdrawRequest from eventsById
    let events_by_id = response["transactionTree"]["eventsById"]
        .as_object()
        .ok_or("Failed to find eventsById in transaction")?;

    for (_key, event) in events_by_id {
        if let Some(created_event) = event.get("CreatedTreeEvent") {
            let template_id = created_event["value"]["templateId"]
                .as_str()
                .unwrap_or("");

            // Match by suffix since template ID can be in different formats
            if template_id.ends_with(":CBTC.WithdrawRequest:CBTCWithdrawRequest") {
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
                return WithdrawRequest::from_active_contract(&active_contract);
            }
        }
    }

    Err("No WithdrawRequest was created in the transaction".to_string())
}

/// List all withdraw requests for a party
///
/// A withdraw request is created after CBTC is burned. Once the attestor network
/// processes the request, BTC will be sent to the destination address and the
/// withdraw request contract will be updated with the Bitcoin transaction ID.
///
/// # Example
/// ```ignore
/// let requests = redeem::list_withdraw_requests(ListWithdrawRequestsParams {
///     ledger_host: "https://participant.example.com".to_string(),
///     party: "party::1220...".to_string(),
///     access_token: "your-token".to_string(),
/// }).await?;
///
/// for request in requests {
///     if let Some(tx_id) = request.btc_tx_id {
///         println!("Withdrawal complete: {} BTC sent in tx {}", request.amount, tx_id);
///     } else {
///         println!("Withdrawal pending: {} BTC to {}", request.amount, request.destination_btc_address);
///     }
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
    let filter = ledger::common::IdentifierFilter::TemplateIdentifierFilter(
        TemplateIdentifierFilter {
            template_filter: TemplateFilter {
                value: TemplateFilterValue {
                    template_id: Some(WITHDRAW_REQUEST_TEMPLATE_ID.to_string()),
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

    let withdraw_requests: Result<Vec<WithdrawRequest>, String> = contracts
        .iter()
        .map(WithdrawRequest::from_active_contract)
        .collect();

    withdraw_requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::login::{password, password_url, PasswordParams};
    use std::env;

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

        println!("Found {} withdraw accounts", accounts.len());
        for account in &accounts {
            println!("  - Contract ID: {}", account.contract_id);
            println!("    Owner: {}", account.owner);
            println!(
                "    Destination BTC Address: {}",
                account.destination_btc_address
            );
        }
    }
}
