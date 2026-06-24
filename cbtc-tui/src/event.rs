use std::time::Duration;

use ratatui::crossterm::event::{self, Event as CtEvent, KeyCode, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{Command, Event, KeyKind};
use crate::config::Profile;
use crate::ops::{self, OpContext, Operation};
use crate::session;

/// Map a raw key code to a semantic `KeyKind`, or `None` to ignore it.
pub fn map_key(code: KeyCode) -> Option<KeyKind> {
    match code {
        KeyCode::Up => Some(KeyKind::Up),
        KeyCode::Down => Some(KeyKind::Down),
        KeyCode::Enter => Some(KeyKind::Enter),
        KeyCode::Tab => Some(KeyKind::Tab),
        KeyCode::PageUp => Some(KeyKind::PageUp),
        KeyCode::PageDown => Some(KeyKind::PageDown),
        KeyCode::Esc => Some(KeyKind::Esc),
        KeyCode::Backspace => Some(KeyKind::Backspace),
        KeyCode::Char(c) => Some(KeyKind::Char(c)),
        _ => None,
    }
}

/// Spawn a blocking thread that reads terminal input and forwards mapped keys.
pub fn spawn_input_reader(tx: UnboundedSender<Event>) {
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            if let Ok(CtEvent::Key(key)) = event::read()
                && key.kind == KeyEventKind::Press
                && let Some(k) = map_key(key.code)
                && tx.send(Event::Key(k)).is_err()
            {
                break;
            }
        } else if tx.is_closed() {
            break;
        }
    });
}

/// Spawn an async task that runs `op` and sends the result back. A panic in the
/// task is converted into an `Event::OpResult(Err(..))` so the UI never hangs.
pub fn spawn_op(tx: UnboundedSender<Event>, op: Operation, ctx: OpContext) {
    let panic_tx = tx.clone();
    let handle = tokio::spawn(async move {
        tracing::info!("running {op} as party {}", ctx.party);
        let result = ops::run(op, &ctx).await.map_err(|e| e.to_string());
        match &result {
            Ok(crate::ops::OpResult::Table { rows, .. }) => {
                tracing::info!("{op}: {} row(s) for party {}", rows.len(), ctx.party);
            }
            Ok(crate::ops::OpResult::Text { .. }) => {
                tracing::info!("{op}: text result for party {}", ctx.party);
            }
            Err(e) => tracing::warn!("{op} failed for party {}: {e}", ctx.party),
        }
        let _ = tx.send(Event::OpResult(result));
    });
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            let _ = panic_tx.send(Event::OpResult(Err(format!("operation panicked: {join_err}"))));
        }
    });
}

/// Spawn an async task that logs in and fetches parties. A panic in the task is
/// converted into an `Event::LoginResult(Err(..))` so the UI never hangs.
pub fn spawn_login(tx: UnboundedSender<Event>, profile: Profile) {
    let panic_tx = tx.clone();
    let handle = tokio::spawn(async move {
        let result = async {
            let s = session::login(&profile).await.map_err(|e| e.to_string())?;
            let parties = session::fetch_parties(&s, &s.access_token)
                .await
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((s.access_token, parties))
        }
        .await;
        match &result {
            Ok((_, parties)) => {
                tracing::info!(
                    "login ok for '{}': {} parties",
                    profile.keycloak_username,
                    parties.len()
                );
            }
            Err(e) => tracing::warn!("login failed for '{}': {e}", profile.keycloak_username),
        }
        let _ = tx.send(Event::LoginResult(result));
    });
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            let _ = panic_tx.send(Event::LoginResult(Err(format!("login panicked: {join_err}"))));
        }
    });
}

/// Spawn an async task that runs a write `command` and sends the result back.
/// A panic in the task is converted into an error so the UI never hangs.
pub fn spawn_command(tx: UnboundedSender<Event>, command: Command, ctx: OpContext) {
    let panic_tx = tx.clone();
    let handle = tokio::spawn(async move {
        tracing::info!("submitting {} for {}", command.verb(), command.cid());
        let result = run_command(&command, &ctx).await;
        match &result {
            Ok(msg) => tracing::info!("command ok: {msg}"),
            Err(e) => tracing::warn!("command failed: {e}"),
        }
        let _ = tx.send(Event::CommandResult(result));
    });
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            let _ = panic_tx.send(Event::CommandResult(Err(format!(
                "command panicked: {join_err}"
            ))));
        }
    });
}

async fn run_command(command: &Command, ctx: &OpContext) -> Result<String, String> {
    match command {
        Command::Accept { cid } => cbtc::accept::submit(cbtc::accept::Params {
            transfer_offer_contract_id: cid.clone(),
            receiver_party: ctx.party.clone(),
            ledger_host: ctx.ledger_host.clone(),
            access_token: ctx.access_token.clone(),
            registry_url: ctx.registry_url.clone(),
            decentralized_party_id: ctx.decentralized_party_id.clone(),
        })
        .await
        .map(|()| "Accepted offer".to_string()),
        Command::Reject { cid } => cbtc::reject::submit(cbtc::reject::Params {
            transfer_offer_contract_id: cid.clone(),
            receiver_party: ctx.party.clone(),
            ledger_host: ctx.ledger_host.clone(),
            access_token: ctx.access_token.clone(),
            registry_url: ctx.registry_url.clone(),
            decentralized_party_id: ctx.decentralized_party_id.clone(),
        })
        .await
        .map(|()| "Rejected offer".to_string()),
        Command::Cancel { cid } => cbtc::cancel_offers::submit(cbtc::cancel_offers::Params {
            transfer_offer_contract_id: cid.clone(),
            sender_party: ctx.party.clone(),
            ledger_host: ctx.ledger_host.clone(),
            access_token: ctx.access_token.clone(),
            registry_url: ctx.registry_url.clone(),
            decentralized_party_id: ctx.decentralized_party_id.clone(),
        })
        .await
        .map(|()| "Cancelled offer".to_string()),
        Command::CancelExpired { cids } => {
            cbtc::cancel_offers::withdraw_batch(cbtc::cancel_offers::WithdrawBatchParams {
                contract_ids: cids.clone(),
                sender_party: ctx.party.clone(),
                ledger_host: ctx.ledger_host.clone(),
                access_token: ctx.access_token.clone(),
                registry_url: ctx.registry_url.clone(),
                decentralized_party_id: ctx.decentralized_party_id.clone(),
            })
            .await
            .map(|res| {
                for r in res.results.iter().filter(|r| !r.success) {
                    tracing::warn!(
                        "cancel-expired failed for {}: {}",
                        r.contract_id,
                        r.error.as_deref().unwrap_or("unknown error")
                    );
                }
                let total = res.successful_count + res.failed_count;
                if res.failed_count == 0 {
                    format!("Cancelled {} expired offer(s)", res.successful_count)
                } else {
                    let first: String = res
                        .results
                        .iter()
                        .find(|r| !r.success)
                        .and_then(|r| r.error.clone())
                        .unwrap_or_default()
                        .chars()
                        .take(80)
                        .collect();
                    format!(
                        "Cancelled {}/{}, {} failed — {first}",
                        res.successful_count, total, res.failed_count
                    )
                }
            })
        }
        Command::MergeHoldings => {
            cbtc::consolidate::consolidate_utxos(cbtc::consolidate::ConsolidateParams {
                party: ctx.party.clone(),
                instrument_id: common::transfer::InstrumentId {
                    admin: ctx.decentralized_party_id.clone(),
                    id: "CBTC".to_string(),
                },
                input_holding_cids: None,
                ledger_host: ctx.ledger_host.clone(),
                access_token: ctx.access_token.clone(),
                registry_url: ctx.registry_url.clone(),
                decentralized_party_id: ctx.decentralized_party_id.clone(),
            })
            .await
            .map(|cids| format!("Merged into {} holding(s)", cids.len()))
        }
        Command::CreateDepositAccount => {
            let rules =
                cbtc::mint_redeem::attestor::get_account_contract_rules(&ctx.bitsafe_api_url).await?;
            let credential_cids = minter_credential_cids(ctx).await?;
            if credential_cids.is_empty() {
                return Err(
                    "No Minter credential found — accept a Minter credential offer first."
                        .to_string(),
                );
            }
            cbtc::mint_redeem::mint::create_deposit_account(
                cbtc::mint_redeem::mint::CreateDepositAccountParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    user_name: ctx.user_name.clone(),
                    access_token: ctx.access_token.clone(),
                    account_rules: rules,
                    credential_cids,
                },
            )
            .await
            .map(|_| "Created deposit account".to_string())
        }
        Command::CreateWithdrawAccount { btc_address } => {
            let rules =
                cbtc::mint_redeem::attestor::get_account_contract_rules(&ctx.bitsafe_api_url).await?;
            let credential_cids = minter_credential_cids(ctx).await?;
            if credential_cids.is_empty() {
                return Err(
                    "No Minter credential found — accept a Minter credential offer first."
                        .to_string(),
                );
            }
            cbtc::mint_redeem::redeem::create_withdraw_account(
                cbtc::mint_redeem::redeem::CreateWithdrawAccountParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    user_name: ctx.user_name.clone(),
                    access_token: ctx.access_token.clone(),
                    account_rules_contract_id: rules.wa_rules.contract_id.clone(),
                    account_rules_template_id: rules.wa_rules.template_id.clone(),
                    account_rules_created_event_blob: rules.wa_rules.created_event_blob.clone(),
                    destination_btc_address: btc_address.clone(),
                    credential_cids,
                },
            )
            .await
            .map(|_| format!("Created withdraw account to {btc_address}"))
        }
        Command::SubmitWithdraw { account_cid, amount } => {
            let amount_dec =
                cbtc::DamlDecimal::parse(amount).map_err(|e| format!("invalid amount: {e}"))?;
            let credential_cids = minter_credential_cids(ctx).await?;
            let holdings = cbtc::mint_redeem::redeem::list_holdings(
                cbtc::mint_redeem::redeem::ListHoldingsParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    access_token: ctx.access_token.clone(),
                },
            )
            .await?;
            // Coin-select CBTC holdings until they cover the amount.
            let mut holding_contract_ids = Vec::new();
            let mut total = cbtc::DamlDecimal::ZERO;
            for h in holdings.iter().filter(|h| h.instrument_id == "CBTC") {
                holding_contract_ids.push(h.contract_id.clone());
                total += h.amount;
                if total >= amount_dec {
                    break;
                }
            }
            if holding_contract_ids.is_empty() {
                return Err("No CBTC holdings available to withdraw".to_string());
            }
            cbtc::mint_redeem::redeem::submit_withdraw(
                cbtc::mint_redeem::redeem::SubmitWithdrawParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    user_name: ctx.user_name.clone(),
                    access_token: ctx.access_token.clone(),
                    api_url: ctx.bitsafe_api_url.clone(),
                    withdraw_account_contract_id: account_cid.clone(),
                    amount: amount_dec,
                    holding_contract_ids,
                    credential_cids: Some(credential_cids),
                },
            )
            .await
            .map(|_| format!("Submitted withdraw of {amount} CBTC"))
        }
    }
}

/// Fetch the party's Minter credential contract ids (CBTC role).
async fn minter_credential_cids(ctx: &OpContext) -> Result<Vec<String>, String> {
    let credentials = cbtc::credentials::list_credentials(cbtc::credentials::ListCredentialsParams {
        ledger_host: ctx.ledger_host.clone(),
        party: ctx.party.clone(),
        access_token: ctx.access_token.clone(),
    })
    .await?;
    Ok(credentials
        .iter()
        .filter(|c| {
            c.claims
                .iter()
                .any(|cl| cl.property == "hasCBTCRole" && cl.value == "Minter")
        })
        .map(|c| c.contract_id.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_to_raw_kinds() {
        use crate::app::KeyKind;
        assert_eq!(map_key(KeyCode::Up), Some(KeyKind::Up));
        assert_eq!(map_key(KeyCode::Enter), Some(KeyKind::Enter));
        assert_eq!(map_key(KeyCode::Tab), Some(KeyKind::Tab));
        assert_eq!(map_key(KeyCode::Esc), Some(KeyKind::Esc));
        assert_eq!(map_key(KeyCode::Backspace), Some(KeyKind::Backspace));
        // Chars map raw — the app interprets them per screen (shortcut vs text).
        assert_eq!(map_key(KeyCode::Char('q')), Some(KeyKind::Char('q')));
        assert_eq!(map_key(KeyCode::Char('z')), Some(KeyKind::Char('z')));
        assert_eq!(map_key(KeyCode::F(1)), None);
    }
}
