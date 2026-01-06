use crate::active_contracts;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Callback function type for handling transfer results
/// Called after each transfer completes (success or failure)
pub type TransferResultCallback =
    dyn Fn(TransferResult) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

pub struct Params {
    pub transfer: common::transfer::Transfer,
    pub ledger_host: String,
    pub access_token: String,
    pub registry_url: String,
    pub decentralized_party_id: String,
}

pub struct MultiParams {
    pub transfers: Vec<common::transfer::Transfer>,
    pub ledger_host: String,
    pub access_token: String,
    pub registry_url: String,
    pub decentralized_party_id: String,
}

#[derive(Clone, Debug)]
pub struct Recipient {
    pub receiver: String,
    pub amount: String,
    pub reference: Option<String>,
}

pub struct SequentialChainedParams {
    pub recipients: Vec<Recipient>,
    pub sender: String,
    pub instrument_id: common::transfer::InstrumentId,
    pub initial_holding_cids: Vec<String>,
    pub ledger_host: String,
    pub registry_url: String,
    pub decentralized_party_id: String,
    // Optional reference base for unique transfer IDs
    pub reference_base: Option<String>,
    // Optional callback for handling each transfer result
    pub on_transfer_complete: Option<Box<TransferResultCallback>>,
    // Optional pre-fetched registry response to reuse context
    pub registry_response: Option<common::transfer_factory::Response>,
}

#[derive(Debug, Clone)]
pub struct TransferResult {
    pub success: bool,
    pub transfer_index: usize,
    pub receiver: String,
    pub amount: String,
    pub transfer_offer_cid: Option<String>,
    pub update_id: Option<String>,
    pub reference: Option<String>,
    pub raw_response: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct SequentialChainedResult {
    pub results: Vec<TransferResult>,
    pub successful_count: usize,
    pub failed_count: usize,
}

/// Simple token state that tracks expiry and refreshes when needed
pub struct TokenState {
    access_token: String,
    refresh_token: String,
    client_id: String,
    url: String,
    username: String,
    password: String,
    expires_at: std::time::SystemTime,
}

impl TokenState {
    pub async fn new(
        username: String,
        password: String,
        client_id: String,
        url: String,
    ) -> Result<Self, String> {
        let token = keycloak::login::password(keycloak::login::PasswordParams {
            client_id: client_id.clone(),
            username: username.clone(),
            password: password.clone(),
            url: url.clone(),
        })
        .await?;

        Ok(TokenState {
            access_token: token.access_token,
            refresh_token: token.refresh_token,

            username,
            password,

            client_id,
            url,
            expires_at: std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(
                    (token.expires_in - 20) as u64,
                ))
                .unwrap_or(std::time::SystemTime::now()),
        })
    }

    /// Get a fresh token, refreshing if needed (within 1 minute of expiry)
    pub async fn get_fresh_token(&mut self) -> Result<String, String> {
        let now = std::time::SystemTime::now();
        let needs_refresh = now >= self.expires_at;

        if needs_refresh {
            let auth = match keycloak::login::refresh(keycloak::login::RefreshParams {
                client_id: self.client_id.clone(),
                refresh_token: self.refresh_token.clone(),
                url: self.url.clone(),
            })
            .await
            {
                Ok(auth_response) => auth_response,
                Err(e) => {
                    if e.contains("Token is not active") {
                        // Try full password login as fallback
                        let auth_response =
                            keycloak::login::password(keycloak::login::PasswordParams {
                                client_id: self.client_id.clone(),
                                username: self.username.clone(),
                                password: self.password.clone(),
                                url: self.url.clone(),
                            })
                            .await?;

                        auth_response
                    } else {
                        return Err(format!("Failed to refresh JWT: {}", e));
                    }
                }
            };

            self.access_token = auth.access_token.clone();
            self.refresh_token = auth.refresh_token;
            // Set expiry to 1 minute before actual expiry
            self.expires_at = std::time::SystemTime::now()
                + std::time::Duration::from_secs(auth.expires_in as u64 - 60);
        }

        Ok(self.access_token.clone())
    }
}

pub async fn submit(mut params: Params) -> Result<(), String> {
    if params.transfer.input_holding_cids.is_none() {
        let contracts = active_contracts::get(active_contracts::Params {
            ledger_host: params.ledger_host.clone(),
            party: params.transfer.sender.clone(),
            access_token: params.access_token.clone(),
        })
        .await?;

        let mut input_holding_cids: Vec<String> = Vec::new();
        for contract in contracts {
            input_holding_cids.push(contract.created_event.contract_id);
        }
        params.transfer.input_holding_cids = Some(input_holding_cids);
    }

    if params.transfer.meta.is_none() {
        let mut transfer_meta: HashMap<String, String> = HashMap::new();
        transfer_meta.insert(
            "splice.lfdecentralizedtrust.org/reason".to_string(),
            "".to_string(),
        );
        params.transfer.meta = Some(common::transfer::Meta {
            values: Some(transfer_meta),
        });
    }

    let additional_information =
        registry::transfer_factory::get(registry::transfer_factory::Params {
            registry_url: params.registry_url,
            decentralized_party_id: params.decentralized_party_id.clone(),
            request: registry::transfer_factory::Request {
                choice_arguments: common::transfer_factory::ChoiceArguments {
                    expected_admin: params.decentralized_party_id.clone(),
                    transfer: params.transfer.clone(),
                    extra_args: common::transfer_factory::ExtraArgs {
                        context: common::transfer_factory::Context {
                            values: HashMap::new(),
                        },
                        meta: common::transfer_factory::Meta {
                            values: common::transfer_factory::MetaValue {},
                        },
                    },
                },
                exclude_debug_fields: true,
            },
        })
        .await?;

    let exercise_command = common::submission::ExerciseCommand {
        exercise_command: common::submission::ExerciseCommandData {
            template_id: common::consts::TEMPLATE_TRANSFER_FACTORY.to_string(),
            contract_id: additional_information.factory_id,
            choice: "TransferFactory_Transfer".to_string(),
            choice_argument: common::submission::ChoiceArgumentsVariations::TransferFactory(
                common::transfer_factory::ChoiceArguments {
                    expected_admin: params.decentralized_party_id,
                    transfer: params.transfer.clone(),
                    extra_args: common::transfer_factory::ExtraArgs {
                        context: additional_information.choice_context.choice_context_data,
                        meta: common::transfer_factory::Meta {
                            values: common::transfer_factory::MetaValue {},
                        },
                    },
                },
            ),
        },
    };

    let submission_request = common::submission::Submission {
        act_as: vec![params.transfer.sender],
        read_as: None,
        command_id: uuid::Uuid::new_v4().to_string(),
        disclosed_contracts: additional_information.choice_context.disclosed_contracts,
        commands: vec![common::submission::Command::ExerciseCommand(
            exercise_command,
        )],
        ..Default::default()
    };

    ledger::submit::wait_for_transaction_tree(ledger::submit::Params {
        ledger_host: params.ledger_host,
        access_token: params.access_token,
        request: submission_request,
    })
    .await?;

    Ok(())
}

/// Submit multiple transfers sequentially, chaining the change output from each transfer
/// as the input for the next transfer. This provides full traceability and partial success.
///
/// The registry context is fetched only once and reused for all transfers, providing
/// significant performance improvement over individual submit() calls.
///
/// Each transfer uses the senderChangeCids from the previous transfer as its input,
/// eliminating the need for pre-splitting UTXOs.
///
/// If JWT refresh credentials are provided, the access token will be automatically
/// refreshed as needed, preventing failures due to token expiration during long operations.
pub async fn submit_sequential_chained(
    params: SequentialChainedParams,
    token_state: &mut TokenState,
) -> Result<SequentialChainedResult, String> {
    if params.recipients.is_empty() {
        return Err("No recipients to process".to_string());
    }

    log::debug!(
        "Starting sequential chained transfers: {} transfers from {}",
        params.recipients.len(),
        params.sender
    );

    let additional_information = match params.registry_response {
        Some(registry_response) => registry_response,
        None => {
            let execute_before_hours = 30 * 24; // 30 days

            // Create a template transfer to fetch registry context
            let template_transfer = common::transfer::Transfer {
                sender: params.sender.clone(),
                receiver: params.recipients[0].receiver.clone(),
                amount: params.recipients[0].amount.clone(),
                instrument_id: params.instrument_id.clone(),
                requested_at: chrono::Utc::now().to_rfc3339(),
                execute_before: (chrono::Utc::now()
                    + chrono::Duration::hours(execute_before_hours))
                .to_rfc3339(),
                input_holding_cids: Some(params.initial_holding_cids.clone()),
                meta: Some(common::transfer::Meta {
                    values: Some({
                        let mut map = HashMap::new();
                        map.insert(
                            "splice.lfdecentralizedtrust.org/reason".to_string(),
                            "".to_string(),
                        );
                        map
                    }),
                }),
            };

            log::debug!("Fetching transfer factory context from registry (once)...");

            let additional_information =
                registry::transfer_factory::get(registry::transfer_factory::Params {
                    registry_url: params.registry_url.clone(),
                    decentralized_party_id: params.decentralized_party_id.clone(),
                    request: registry::transfer_factory::Request {
                        choice_arguments: common::transfer_factory::ChoiceArguments {
                            expected_admin: params.decentralized_party_id.clone(),
                            transfer: template_transfer,
                            extra_args: common::transfer_factory::ExtraArgs {
                                context: common::transfer_factory::Context {
                                    values: HashMap::new(),
                                },
                                meta: common::transfer_factory::Meta {
                                    values: common::transfer_factory::MetaValue {},
                                },
                            },
                        },
                        exclude_debug_fields: true,
                    },
                })
                .await?;
            additional_information
        }
    };

    let factory_id = additional_information.factory_id;
    let choice_context_data = additional_information.choice_context.choice_context_data;
    let disclosed_contracts = additional_information.choice_context.disclosed_contracts;

    log::debug!("Registry context fetched successfully");

    // Track results and current holdings
    let mut results = Vec::new();
    let mut current_holding_cids = params.initial_holding_cids;
    let mut successful_count = 0;
    let mut failed_count = 0;

    let total_transfers = params.recipients.len();

    // Process each recipient sequentially, building transfers on-the-fly
    for (idx, recipient) in params.recipients.into_iter().enumerate() {
        let transfer_num = idx + 1;
        log::trace!(
            "\n[{}/{}] Transferring {} to {}...",
            transfer_num,
            total_transfers,
            recipient.amount,
            recipient.receiver
        );

        if current_holding_cids.is_empty() {
            let error_msg = "No UTXOs available for transfer".to_string();
            log::error!("{}", error_msg);
            let result = TransferResult {
                success: false,
                transfer_index: idx,
                receiver: recipient.receiver.clone(),
                amount: recipient.amount.clone(),
                transfer_offer_cid: None,
                update_id: None,
                reference: None,
                raw_response: None,
                error: Some(error_msg),
            };

            // Call callback if provided
            if let Some(ref callback) = params.on_transfer_complete {
                callback(result.clone()).await;
            }

            results.push(result);
            failed_count += 1;
            continue;
        }

        // Get fresh JWT token (auto-refreshes if expired)
        let current_token = match token_state.get_fresh_token().await {
            Ok(token) => token,
            Err(e) => {
                let error_msg = format!("Failed to get fresh token: {}", e);
                log::error!("{}", error_msg);

                let result = TransferResult {
                    success: false,
                    transfer_index: idx,
                    receiver: recipient.receiver.clone(),
                    amount: recipient.amount.clone(),
                    transfer_offer_cid: None,
                    update_id: None,
                    reference: None,
                    raw_response: None,
                    error: Some(error_msg),
                };

                // Call callback if provided
                if let Some(ref callback) = params.on_transfer_complete {
                    callback(result.clone()).await;
                }

                results.push(result);
                failed_count += 1;
                continue;
            }
        };

        // Generate unique reference if reference_base is provided
        let mut transfer_reference = params.reference_base.as_ref().map(|reference_base| {
            generate_unique_reference(reference_base, &params.sender, &recipient.receiver)
        });
        if let Some(ref unique_ref) = recipient.reference {
            transfer_reference = Some(unique_ref.clone());
        }

        // Build transfer on-the-fly for this recipient
        let transfer = common::transfer::Transfer {
            sender: params.sender.clone(),
            receiver: recipient.receiver.clone(),
            amount: recipient.amount.clone(),
            instrument_id: params.instrument_id.clone(),
            requested_at: chrono::Utc::now().to_rfc3339(),
            execute_before: (chrono::Utc::now() + chrono::Duration::hours(168)).to_rfc3339(),
            input_holding_cids: Some(current_holding_cids.clone()),
            meta: Some(common::transfer::Meta {
                values: Some({
                    let mut map = HashMap::new();
                    map.insert(
                        "splice.lfdecentralizedtrust.org/reason".to_string(),
                        "".to_string(),
                    );

                    // Add unique reference ID if generated
                    if let Some(ref unique_ref) = transfer_reference {
                        map.insert(
                            "splice.lfdecentralizedtrust.org/reference".to_string(),
                            unique_ref.clone(),
                        );
                    }

                    map
                }),
            }),
        };

        // Create exercise command using the shared factory context
        let exercise_command = common::submission::ExerciseCommand {
            exercise_command: common::submission::ExerciseCommandData {
                template_id: common::consts::TEMPLATE_TRANSFER_FACTORY.to_string(),
                contract_id: factory_id.clone(),
                choice: "TransferFactory_Transfer".to_string(),
                choice_argument: common::submission::ChoiceArgumentsVariations::TransferFactory(
                    common::transfer_factory::ChoiceArguments {
                        expected_admin: params.decentralized_party_id.clone(),
                        transfer: transfer.clone(),
                        extra_args: common::transfer_factory::ExtraArgs {
                            context: choice_context_data.clone(),
                            meta: common::transfer_factory::Meta {
                                values: common::transfer_factory::MetaValue {},
                            },
                        },
                    },
                ),
            },
        };

        let submission_request = common::submission::Submission {
            act_as: vec![params.sender.clone()],
            read_as: None,
            command_id: uuid::Uuid::new_v4().to_string(),
            disclosed_contracts: disclosed_contracts.clone(),
            commands: vec![common::submission::Command::ExerciseCommand(
                exercise_command,
            )],
            ..Default::default()
        };

        // Submit to ledger with fresh token
        match ledger::submit::wait_for_transaction_tree(ledger::submit::Params {
            ledger_host: params.ledger_host.clone(),
            access_token: current_token,
            request: submission_request,
        })
        .await
        {
            Ok(response_raw) => {
                // Parse response to extract change UTXOs, transfer offer CID, and update_id
                match parse_transfer_response(&response_raw) {
                    Ok((sender_change_cids, transfer_offer_cid, update_id)) => {
                        log::debug!(
                            "Transfer successful | Transfer Offer: {} | Update ID: {} | Change UTXOs: {} remaining",
                            transfer_offer_cid,
                            update_id,
                            sender_change_cids.len()
                        );

                        let result = TransferResult {
                            success: true,
                            transfer_index: idx,
                            receiver: recipient.receiver.clone(),
                            amount: recipient.amount.clone(),
                            transfer_offer_cid: Some(transfer_offer_cid),
                            update_id: Some(update_id),
                            reference: transfer_reference.clone(),
                            raw_response: Some(response_raw.clone()),
                            error: None,
                        };

                        // Call callback if provided
                        if let Some(ref callback) = params.on_transfer_complete {
                            callback(result.clone()).await;
                        }

                        results.push(result);
                        successful_count += 1;

                        // Use change as input for next transfer
                        current_holding_cids = sender_change_cids;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to parse transfer response: {}", e);
                        log::error!(
                            "{} | Note: Change UTXOs preserved for next transfer",
                            error_msg
                        );
                        let result = TransferResult {
                            success: false,
                            transfer_index: idx,
                            receiver: recipient.receiver.clone(),
                            amount: recipient.amount.clone(),
                            transfer_offer_cid: None,
                            update_id: None,
                            reference: transfer_reference.clone(),
                            raw_response: Some(response_raw),
                            error: Some(error_msg),
                        };

                        // Call callback if provided
                        if let Some(ref callback) = params.on_transfer_complete {
                            callback(result.clone()).await;
                        }

                        results.push(result);
                        failed_count += 1;
                        // Keep current_holding_cids - we can still try the next transfer
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Ledger submission failed: {}", e);
                log::error!(
                    "{} | Note: Change UTXOs preserved, will retry next transfer",
                    error_msg
                );
                let result = TransferResult {
                    success: false,
                    transfer_index: idx,
                    receiver: recipient.receiver.clone(),
                    amount: recipient.amount.clone(),
                    transfer_offer_cid: None,
                    update_id: None,
                    reference: transfer_reference.clone(),
                    raw_response: None, // No response on submission failure
                    error: Some(error_msg),
                };

                // Call callback if provided
                if let Some(ref callback) = params.on_transfer_complete {
                    callback(result.clone()).await;
                }

                results.push(result);
                failed_count += 1;
                // Keep current_holding_cids - the UTXOs are still valid
            }
        }
    }

    log::debug!(
        "Transfer Summary: Successful: {}, Failed: {}",
        successful_count,
        failed_count
    );

    Ok(SequentialChainedResult {
        results,
        successful_count,
        failed_count,
    })
}

/// Parse the transfer response to extract sender change CIDs, transfer offer CID, and update_id
pub fn parse_transfer_response(
    response_raw: &str,
) -> Result<(Vec<String>, String, String), String> {
    let response: serde_json::Value = serde_json::from_str(response_raw)
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    // Extract update_id from the root level
    let update_id = response["transactionTree"]["updateId"]
        .as_str()
        .ok_or("Failed to find updateId in response")?
        .to_string();

    let events_by_id = response["transactionTree"]["eventsById"]
        .as_object()
        .ok_or("Failed to find eventsById in response")?;

    // Find the ExercisedTreeEvent with TransferFactory_Transfer choice
    let mut sender_change_cids = None;
    let mut transfer_offer_cid = None;

    for (_key, event) in events_by_id {
        if let Some(exercised_event) = event.get("ExercisedTreeEvent") {
            let choice = exercised_event["value"]["choice"].as_str();
            if choice == Some("TransferFactory_Transfer") {
                // Extract senderChangeCids
                if let Some(change_array) =
                    exercised_event["value"]["exerciseResult"]["senderChangeCids"].as_array()
                {
                    sender_change_cids = Some(
                        change_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>(),
                    );
                }

                // Extract transfer offer CID from the output
                if let Some(output) = exercised_event["value"]["exerciseResult"]["output"]["value"]
                    ["transferInstructionCid"]
                    .as_str()
                {
                    transfer_offer_cid = Some(output.to_string());
                }
            }
        }
    }

    let sender_change_cids =
        sender_change_cids.ok_or("Failed to find senderChangeCids in response")?;
    let transfer_offer_cid =
        transfer_offer_cid.ok_or("Failed to find transferInstructionCid in response")?;

    Ok((sender_change_cids, transfer_offer_cid, update_id))
}

/// Generate a unique reference by concatenating reference_base + sender + receiver and base64 encoding
fn generate_unique_reference(reference_base: &str, sender: &str, receiver: &str) -> String {
    use base64::{Engine as _, engine::general_purpose};

    let combined = format!("{}-{}-{}", reference_base, sender, receiver);
    general_purpose::STANDARD.encode(combined.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycloak::login::{PasswordParams, password, password_url};
    use std::env;
    use std::ops::Add;

    #[tokio::test]
    async fn test_submit() {
        // Load environment variables from .env file
        dotenvy::dotenv().ok();

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

        let sender_party = env::var("PARTY_ID").expect("PARTY_ID must be set");
        let receiver_party =
            env::var("LIB_TEST_RECEIVER_PARTY_ID").expect("LIB_TEST_RECEIVER_PARTY_ID must be set");
        let decentralized_party =
            env::var("DECENTRALIZED_PARTY_ID").expect("DECENTRALIZED_PARTY_ID must be set");

        let params = Params {
            transfer: common::transfer::Transfer {
                sender: sender_party,
                receiver: receiver_party,
                amount: "0.02".to_string(),
                instrument_id: common::transfer::InstrumentId {
                    admin: decentralized_party.clone(),
                    id: "CBTC".to_string(),
                },
                requested_at: chrono::Utc::now().to_rfc3339(),
                execute_before: chrono::Utc::now()
                    .add(chrono::Duration::hours(168))
                    .to_rfc3339(),
                input_holding_cids: None,
                meta: None,
            },
            ledger_host: env::var("LEDGER_HOST").expect("LEDGER_HOST must be set"),
            access_token: login_response.access_token,
            registry_url: env::var("REGISTRY_URL").expect("REGISTRY_URL must be set"),
            decentralized_party_id: decentralized_party,
        };

        submit(params).await.unwrap();
    }
}
