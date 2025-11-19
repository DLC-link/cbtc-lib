/// Extract amount from a contract's interface views
pub fn extract_amount(contract: &ledger::models::JsActiveContract) -> Option<f64> {
    if let Some(views) = &contract.created_event.interface_views {
        for view in views {
            if let Some(Some(value)) = &view.view_value {
                if let Some(amount_value) = value.get("amount") {
                    if let Some(amount_str) = amount_value.as_str() {
                        return amount_str.parse::<f64>().ok();
                    } else if let Some(amount_f64) = amount_value.as_f64() {
                        return Some(amount_f64);
                    }
                }
            }
        }
    }
    None
}

/// Fetch all pending CBTC TransferInstruction contracts for a party where the party is the receiver
pub async fn fetch_incoming_transfers(
    ledger_host: String,
    party: String,
    access_token: String,
) -> Result<Vec<ledger::models::JsActiveContract>, String> {
    fetch_transfers(
        ledger_host,
        party,
        access_token,
        TransferDirection::Incoming,
    )
    .await
}

/// Fetch all pending CBTC TransferInstruction contracts for a party where the party is the sender
pub async fn fetch_outgoing_transfers(
    ledger_host: String,
    party: String,
    access_token: String,
) -> Result<Vec<ledger::models::JsActiveContract>, String> {
    fetch_transfers(
        ledger_host,
        party,
        access_token,
        TransferDirection::Outgoing,
    )
    .await
}

enum TransferDirection {
    Incoming,
    Outgoing,
}

/// Fetch all pending CBTC TransferInstruction contracts for a party
async fn fetch_transfers(
    ledger_host: String,
    party: String,
    access_token: String,
    direction: TransferDirection,
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

    // Filter for CBTC transfers based on direction
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

                    // Check role based on direction
                    let matches_direction = match direction {
                        TransferDirection::Incoming => {
                            // Check if we are the receiver
                            if let Some(receiver) = transfer.get("receiver") {
                                if let Some(receiver_str) = receiver.as_str() {
                                    receiver_str == party
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        TransferDirection::Outgoing => {
                            // Check if we are the sender
                            if let Some(sender) = transfer.get("sender") {
                                if let Some(sender_str) = sender.as_str() {
                                    sender_str == party
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    };

                    return is_cbtc && matches_direction;
                }
            }
            false
        })
        .collect();

    Ok(filtered)
}
