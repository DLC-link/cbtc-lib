/// Parameters for accepting a transfer.
/// The receiver party must provide authentication to accept the transfer.
pub struct Params {
    /// The contract ID of the TransferOffer/TransferInstruction to accept
    pub transfer_offer_contract_id: String,
    /// The receiver party ID (must match the transfer's receiver)
    pub receiver_party: String,
    /// Ledger host URL
    pub ledger_host: String,
    /// Access token for the receiver party
    pub access_token: String,
    /// Registry URL
    pub registry_url: String,
    /// Decentralized party ID for CBTC
    pub decentralized_party_id: String,
}

/// Parameters for accepting all pending CBTC transfers for a party.
pub struct AcceptAllParams {
    /// The receiver party ID
    pub receiver_party: String,
    /// Ledger host URL
    pub ledger_host: String,
    /// Registry URL
    pub registry_url: String,
    /// Decentralized party ID for CBTC
    pub decentralized_party_id: String,
    // Keycloak authentication
    pub keycloak_client_id: String,
    pub keycloak_username: String,
    pub keycloak_password: String,
    pub keycloak_url: String,
}

/// Result of accepting a single transfer
#[derive(Debug, Clone)]
pub struct AcceptResult {
    pub success: bool,
    pub contract_id: String,
    pub amount: Option<String>,
    pub sender: Option<String>,
    pub error: Option<String>,
}

/// Result of accepting all pending transfers
#[derive(Debug)]
pub struct AcceptAllResult {
    pub results: Vec<AcceptResult>,
    pub successful_count: usize,
    pub failed_count: usize,
}

/// Accept a CBTC transfer as the receiving party.
///
/// This function performs the following steps:
/// 1. Fetches the choice context from the registry for accepting the transfer
/// 2. Constructs the exercise command for TransferInstruction_Accept
/// 3. Submits the transaction to the ledger
///
/// # Example
/// ```ignore
/// use cbtc::accept;
///
/// let params = accept::Params {
///     transfer_offer_contract_id: "00abc123...".to_string(),
///     receiver_party: "receiver-party::1220...".to_string(),
///     ledger_host: "https://participant.example.com".to_string(),
///     access_token: "eyJ...".to_string(),
///     registry_url: "https://api.utilities.digitalasset-dev.com".to_string(),
///     decentralized_party_id: "cbtc-network::1220...".to_string(),
/// };
///
/// accept::submit(params).await?;
/// ```
pub async fn submit(params: Params) -> Result<(), String> {
    // Get the choice context for accepting the transfer from the registry
    let accept_context = registry::accept_context::get(registry::accept_context::Params {
        registry_url: params.registry_url,
        decentralized_party_id: params.decentralized_party_id.clone(),
        transfer_offer_contract_id: params.transfer_offer_contract_id.clone(),
        request: registry::accept_context::Request {
            meta: registry::accept_context::Meta {
                values: String::new(),
            },
        },
    })
    .await?;

    // Construct the exercise command to accept the transfer
    let exercise_command = common::submission::ExerciseCommand {
        exercise_command: common::submission::ExerciseCommandData {
            template_id: common::consts::TEMPLATE_TRANSFER_INSTRUCTION.to_string(),
            contract_id: params.transfer_offer_contract_id,
            choice: "TransferInstruction_Accept".to_string(),
            choice_argument: common::submission::ChoiceArgumentsVariations::Accept(
                common::accept::ChoiceArguments {
                    extra_args: common::accept::ExtraArgs {
                        context: common::accept::Context {
                            values: accept_context.choice_context_data.values,
                        },
                        meta: common::accept::Meta {
                            values: common::accept::MetaValue {},
                        },
                    },
                },
            ),
        },
    };

    // Submit the acceptance transaction
    let submission_request = common::submission::Submission {
        act_as: vec![params.receiver_party],
        command_id: uuid::Uuid::new_v4().to_string(),
        disclosed_contracts: accept_context.disclosed_contracts,
        commands: vec![common::submission::Command::ExerciseCommand(
            exercise_command,
        )],
        read_as: None,
        user_id: None,
    };

    ledger::submit::wait_for_transaction_tree(ledger::submit::Params {
        ledger_host: params.ledger_host,
        access_token: params.access_token,
        request: submission_request,
    })
    .await?;

    Ok(())
}

/// Accept all pending CBTC transfers for a party.
///
/// This function:
/// 1. Authenticates with Keycloak
/// 2. Fetches all pending TransferInstruction contracts for the party
/// 3. Filters for CBTC transfers where the party is the receiver
/// 4. Accepts each transfer sequentially
///
/// Returns a summary of successful and failed acceptances.
pub async fn accept_all(params: AcceptAllParams) -> Result<AcceptAllResult, String> {
    log::debug!("Authenticating with Keycloak...");
    let auth = keycloak::login::password(keycloak::login::PasswordParams {
        client_id: params.keycloak_client_id,
        username: params.keycloak_username,
        password: params.keycloak_password,
        url: params.keycloak_url,
    })
    .await
    .map_err(|e| format!("Authentication failed: {}", e))?;

    log::debug!("Authenticated successfully");

    log::debug!(
        "Checking for pending transfers for party: {}",
        params.receiver_party
    );

    // Fetch pending transfer instructions
    let pending_transfers = fetch_pending_transfers(
        params.ledger_host.clone(),
        params.receiver_party.clone(),
        auth.access_token.clone(),
    )
    .await?;

    if pending_transfers.is_empty() {
        log::debug!("No pending transfers found");
        return Ok(AcceptAllResult {
            results: Vec::new(),
            successful_count: 0,
            failed_count: 0,
        });
    }

    log::debug!("Found {} pending transfer(s)", pending_transfers.len());

    // Accept each transfer
    let mut results = Vec::new();
    let mut successful_count = 0;
    let mut failed_count = 0;

    for (idx, transfer) in pending_transfers.iter().enumerate() {
        let contract_id = &transfer.created_event.contract_id;
        let short_id = if contract_id.len() > 16 {
            format!(
                "{}...{}",
                &contract_id[..8],
                &contract_id[contract_id.len() - 8..]
            )
        } else {
            contract_id.clone()
        };

        log::debug!("{}. Accepting transfer {}", idx + 1, short_id);

        // Extract transfer details from create_argument
        let mut amount = None;
        let mut sender = None;

        if let Some(Some(create_arg)) = &transfer.created_event.create_argument {
            if let Some(transfer_data) = create_arg.get("transfer") {
                if let Some(amt) = transfer_data.get("amount") {
                    amount = amt.as_str().map(|s| s.to_string());
                    log::debug!("Amount: {}", amt);
                }
                if let Some(sndr) = transfer_data.get("sender") {
                    sender = sndr.as_str().map(|s| s.to_string());
                    log::debug!("From: {}", sndr.as_str().unwrap_or("unknown"));
                }
            }
        }

        // Accept the transfer
        let accept_params = Params {
            transfer_offer_contract_id: contract_id.clone(),
            receiver_party: params.receiver_party.clone(),
            ledger_host: params.ledger_host.clone(),
            access_token: auth.access_token.clone(),
            registry_url: params.registry_url.clone(),
            decentralized_party_id: params.decentralized_party_id.clone(),
        };

        match submit(accept_params).await {
            Ok(_) => {
                log::debug!("Accepted");
                results.push(AcceptResult {
                    success: true,
                    contract_id: contract_id.clone(),
                    amount: amount.clone(),
                    sender: sender.clone(),
                    error: None,
                });
                successful_count += 1;
            }
            Err(e) => {
                log::debug!("Failed: {}", e);
                results.push(AcceptResult {
                    success: false,
                    contract_id: contract_id.clone(),
                    amount: amount.clone(),
                    sender: sender.clone(),
                    error: Some(e),
                });
                failed_count += 1;
            }
        }
    }

    log::debug!("Summary:");
    log::debug!("Accepted: {}", successful_count);
    if failed_count > 0 {
        log::debug!("Failed: {}", failed_count);
    }

    Ok(AcceptAllResult {
        results,
        successful_count,
        failed_count,
    })
}

/// Fetch all pending TransferInstruction contracts for a party
async fn fetch_pending_transfers(
    ledger_host: String,
    party: String,
    access_token: String,
) -> Result<Vec<ledger::models::JsActiveContract>, String> {
    use ledger::ledger_end;
    use ledger::websocket::active_contracts;

    // Get current ledger end
    let ledger_end_result = ledger_end::get(ledger_end::Params {
        access_token: access_token.clone(),
        ledger_host: ledger_host.clone(),
    })
    .await?;

    // Fetch all active contracts with TransferInstruction template filter
    let result = active_contracts::get(active_contracts::Params {
        ledger_host,
        party: party.clone(),
        filter: ledger::common::IdentifierFilter::TemplateIdentifierFilter(
            ledger::common::TemplateIdentifierFilter {
                template_filter: ledger::common::TemplateFilter {
                    value: ledger::common::TemplateFilterValue {
                        template_id: Some(common::consts::TEMPLATE_TRANSFER_OFFER.to_string()),
                        include_created_event_blob: true,
                    },
                },
            },
        ),
        access_token,
        ledger_end: ledger_end_result.offset,
    })
    .await?;

    log::debug!(
        "Total active TransferInstruction contracts fetched: {}",
        result.len()
    );

    // Filter for CBTC transfers where this party is the receiver
    let filtered: Vec<ledger::models::JsActiveContract> = result
        .into_iter()
        .filter(|ac| {
            if let Some(Some(create_arg)) = &ac.created_event.create_argument {
                if let Some(transfer) = create_arg.get("transfer") {
                    // Check if instrumentId is CBTC
                    let is_cbtc = if let Some(instrument_id) = transfer.get("instrumentId") {
                        if let Some(id) = instrument_id.get("id") {
                            if let Some(id_str) = id.as_str() {
                                id_str.to_lowercase() == "cbtc"
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Check if we are the receiver
                    let is_receiver = if let Some(receiver) = transfer.get("receiver") {
                        if let Some(receiver_str) = receiver.as_str() {
                            receiver_str == party
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    return is_cbtc && is_receiver;
                }
            }
            false
        })
        .collect();

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_accept_transfer() {
        // This test requires a valid transfer_offer_contract_id from an actual transfer
        // Load environment variables from .env file
        dotenvy::dotenv().ok();

        let transfer_offer_cid = std::env::var("LIB_TEST_TRANSFER_OFFER_CID").ok();

        if transfer_offer_cid.is_none() {
            return;
        }

        // Note: This would require:
        // 1. A valid transfer_offer_contract_id from a pending transfer
        // 2. Authentication as the receiver party
        // 3. The transfer must be in a state ready for acceptance
    }
}
