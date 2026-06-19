use std::sync::Arc;

use serde_json::Value;
use strum::{Display, EnumIter};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::error::{AppError, Result};

/// The read-only query operations. `EnumIter` builds the menu; `Display` labels it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Display)]
pub enum Operation {
    #[strum(serialize = "Check Balance")]
    CheckBalance,
    #[strum(serialize = "Incoming Offers")]
    IncomingOffers,
    #[strum(serialize = "Outgoing Offers")]
    OutgoingOffers,
    #[strum(serialize = "Deposit Addresses")]
    DepositAddresses,
    #[strum(serialize = "Withdraw Accounts")]
    WithdrawAccounts,
    #[strum(serialize = "Withdraw Requests")]
    WithdrawRequests,
    #[strum(serialize = "DAR Versions")]
    DarVersions,
    #[strum(serialize = "Credentials")]
    Credentials,
}

/// One row of a table result: display `cells`, an optional `detail` payload
/// (shown in the detail view), an optional `id` (full contract id used as the
/// target for row actions), and an `expired` flag for offer rows past their
/// `executeBefore`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultRow {
    pub cells: Vec<String>,
    pub detail: Option<String>,
    pub id: Option<String>,
    pub expired: bool,
}

impl ResultRow {
    pub fn new(cells: Vec<String>, detail: Option<String>) -> Self {
        Self { cells, detail, id: None, expired: false }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_expired(mut self, expired: bool) -> Self {
        self.expired = expired;
        self
    }
}

/// Normalized result shape the UI renders generically.
// Text variant is rendered by ui.rs and constructed in tests; ops currently only
// produce Table results, so rustc flags it as dead — allow for extensibility.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum OpResult {
    Table {
        title: String,
        columns: Vec<String>,
        rows: Vec<ResultRow>,
    },
    Text {
        title: String,
        body: String,
    },
}

/// Everything an operation needs to run, assembled from the active profile,
/// environment, party, and session token.
#[derive(Debug, Clone)]
pub struct OpContext {
    pub ledger_host: String,
    pub party: String,
    pub access_token: String,
    pub bitsafe_api_url: String,
    pub registry_url: String,
    pub decentralized_party_id: String,
    pub user_name: String,
    pub dar_dirs: Vec<String>,
}

/// A flattened transfer-offer row extracted from raw contract JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRow {
    pub counterparty: String,
    pub amount: String,
    pub requested_at: String,
    pub execute_before: String,
}

/// Short-id helper: `00aabb…eeff`. Truncate to `head…tail` only when longer than 14 chars; char-safe.
fn short(id: &str) -> String {
    let count = id.chars().count();
    if count > 14 {
        let head: String = id.chars().take(6).collect();
        let tail: String = id.chars().skip(count - 6).collect();
        format!("{head}…{tail}")
    } else {
        id.to_string()
    }
}

/// Detail payload for a ledger contract: the full created event (offset,
/// `createdAt`, `templateId`, signatories, `createArgument`, …) minus the bulky
/// `createdEventBlob`.
fn contract_detail(c: &ledger::models::JsActiveContract) -> Option<String> {
    let mut v = serde_json::to_value(&c.created_event).ok()?;
    if let Some(obj) = v.as_object_mut() {
        obj.remove("createdEventBlob");
    }
    serde_json::to_string_pretty(&v).ok()
}

/// Build a balance table from pre-extracted (contract_id, amount, detail) rows.
pub fn balance_to_result(rows: &[(String, cbtc::DamlDecimal, Option<String>)]) -> OpResult {
    let total: cbtc::DamlDecimal = rows
        .iter()
        .map(|(_, a, _)| *a)
        .fold(cbtc::DamlDecimal::ZERO, |acc, a| acc + a);
    let table_rows = rows
        .iter()
        .enumerate()
        .map(|(i, (id, amount, detail))| {
            ResultRow::new(
                vec![(i + 1).to_string(), amount.to_string(), short(id)],
                detail.clone(),
            )
        })
        .collect();
    OpResult::Table {
        title: format!("Total CBTC: {total}  ({} UTXOs)", rows.len()),
        columns: vec!["#".to_string(), "Amount".to_string(), "Contract".to_string()],
        rows: table_rows,
    }
}

/// Extract a transfer row from `created_event.create_argument`. `counterparty_key`
/// is `"sender"` (incoming) or `"receiver"` (outgoing).
pub fn parse_transfer_row(arg: &Value, counterparty_key: &str) -> Option<TransferRow> {
    let t = arg.get("transfer")?;
    let s = |k: &str| t.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Some(TransferRow {
        counterparty: s(counterparty_key),
        amount: s("amount"),
        requested_at: s("requestedAt"),
        execute_before: s("executeBefore"),
    })
}

fn transfers_to_result(
    contracts: &[ledger::models::JsActiveContract],
    counterparty_key: &str,
    counterparty_label: &str,
    title: &str,
) -> OpResult {
    let rows = contracts
        .iter()
        .filter_map(|c| {
            let arg = c.created_event.create_argument.as_ref()?;
            let r = parse_transfer_row(arg, counterparty_key)?;
            let expired = chrono::DateTime::parse_from_rfc3339(&r.execute_before)
                .map(|t| t.with_timezone(&chrono::Utc) < chrono::Utc::now())
                .unwrap_or(false);
            Some(
                ResultRow::new(
                    vec![
                        r.counterparty,
                        r.amount,
                        r.execute_before,
                        short(&c.created_event.contract_id),
                    ],
                    contract_detail(c),
                )
                .with_id(c.created_event.contract_id.clone())
                .with_expired(expired),
            )
        })
        .collect();
    OpResult::Table {
        title: title.to_string(),
        columns: vec![
            counterparty_label.to_string(),
            "Amount".to_string(),
            "Expires".to_string(),
            "Contract".to_string(),
        ],
        rows,
    }
}

/// Run an operation and normalize its result.
///
/// # Errors
/// Returns `AppError::Op` if the underlying `cbtc` call fails.
pub async fn run(op: Operation, ctx: &OpContext) -> Result<OpResult> {
    match op {
        Operation::CheckBalance => {
            let holdings = cbtc::active_contracts::get(cbtc::active_contracts::Params {
                ledger_host: ctx.ledger_host.clone(),
                party: ctx.party.clone(),
                access_token: ctx.access_token.clone(),
            })
            .await
            .map_err(AppError::Op)?;
            let rows: Vec<(String, cbtc::DamlDecimal, Option<String>)> = holdings
                .iter()
                .map(|c| {
                    (
                        c.created_event.contract_id.clone(),
                        cbtc::utils::extract_amount(c).unwrap_or(cbtc::DamlDecimal::ZERO),
                        contract_detail(c),
                    )
                })
                .collect();
            Ok(balance_to_result(&rows))
        }
        Operation::IncomingOffers => {
            let c = cbtc::utils::fetch_incoming_transfers(
                ctx.ledger_host.clone(),
                ctx.party.clone(),
                ctx.access_token.clone(),
            )
            .await
            .map_err(AppError::Op)?;
            Ok(transfers_to_result(&c, "sender", "From", "Incoming Offers"))
        }
        Operation::OutgoingOffers => {
            let c = cbtc::utils::fetch_outgoing_transfers(
                ctx.ledger_host.clone(),
                ctx.party.clone(),
                ctx.access_token.clone(),
            )
            .await
            .map_err(AppError::Op)?;
            Ok(transfers_to_result(&c, "receiver", "To", "Outgoing Offers"))
        }
        Operation::DepositAddresses => {
            let accounts = cbtc::mint_redeem::mint::list_deposit_accounts(
                cbtc::mint_redeem::mint::ListDepositAccountsParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    access_token: ctx.access_token.clone(),
                },
            )
            .await
            .map_err(AppError::Op)?;
            // Fetch each account's BTC address concurrently (bounded), preserving
            // order — a user with many deposit accounts shouldn't wait on N serial
            // round-trips.
            let semaphore = Arc::new(Semaphore::new(8));
            let mut set = JoinSet::new();
            for (i, a) in accounts.iter().enumerate() {
                let api_url = ctx.bitsafe_api_url.clone();
                let account_id = a.account_id().to_string();
                let semaphore = semaphore.clone();
                set.spawn(async move {
                    let _permit = semaphore.acquire().await.ok();
                    let addr = cbtc::mint_redeem::mint::get_bitcoin_address(
                        cbtc::mint_redeem::mint::GetBitcoinAddressParams { api_url, account_id },
                    )
                    .await
                    .unwrap_or_else(|e| format!("<error: {e}>"));
                    (i, addr)
                });
            }
            let mut addresses = vec![String::from("<error: task failed>"); accounts.len()];
            while let Some(joined) = set.join_next().await {
                if let Ok((i, addr)) = joined {
                    addresses[i] = addr;
                }
            }
            let rows = accounts
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    ResultRow::new(
                        vec![a.account_id().to_string(), addresses[i].clone(), short(&a.contract_id)],
                        Some(format!("{a:#?}\n\nbitcoin_address: {}", addresses[i])),
                    )
                })
                .collect();
            Ok(OpResult::Table {
                title: format!("Deposit Accounts ({})", accounts.len()),
                columns: vec!["Account".into(), "BTC Address".into(), "Contract".into()],
                rows,
            })
        }
        Operation::WithdrawAccounts => {
            let accts = cbtc::mint_redeem::redeem::list_withdraw_accounts(
                cbtc::mint_redeem::redeem::ListWithdrawAccountsParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    access_token: ctx.access_token.clone(),
                },
            )
            .await
            .map_err(AppError::Op)?;
            let rows = accts
                .iter()
                .map(|a| {
                    let status = if a.pending_balance > cbtc::DamlDecimal::ZERO {
                        "PENDING"
                    } else {
                        "ready"
                    };
                    ResultRow::new(
                        vec![
                            a.destination_btc_address.clone(),
                            a.pending_balance.to_string(),
                            status.to_string(),
                            short(&a.contract_id),
                        ],
                        Some(format!("{a:#?}")),
                    )
                    .with_id(a.contract_id.clone())
                })
                .collect();
            Ok(OpResult::Table {
                title: format!("Withdraw Accounts ({})", accts.len()),
                columns: vec![
                    "Destination".into(),
                    "Pending".into(),
                    "Status".into(),
                    "Contract".into(),
                ],
                rows,
            })
        }
        Operation::WithdrawRequests => {
            let reqs = cbtc::mint_redeem::redeem::list_withdraw_requests(
                cbtc::mint_redeem::redeem::ListWithdrawRequestsParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    access_token: ctx.access_token.clone(),
                },
            )
            .await
            .map_err(AppError::Op)?;
            let rows = reqs
                .iter()
                .map(|r| {
                    ResultRow::new(
                        vec![
                            r.amount.to_string(),
                            r.destination_btc_address.clone(),
                            r.btc_tx_id.clone(),
                            short(&r.contract_id),
                        ],
                        Some(format!("{r:#?}")),
                    )
                })
                .collect();
            Ok(OpResult::Table {
                title: format!("Withdraw Requests ({})", reqs.len()),
                columns: vec![
                    "Amount".into(),
                    "Destination".into(),
                    "BTC Tx".into(),
                    "Contract".into(),
                ],
                rows,
            })
        }
        Operation::DarVersions => {
            if ctx.dar_dirs.is_empty() {
                return Ok(OpResult::Text {
                    title: "DAR Versions".to_string(),
                    body: "No DAR directories are configured, so there is nothing to validate.\n\
                           Configure local DAR paths to compare installed packages against the \
                           participant."
                        .to_string(),
                });
            }
            let result = cbtc::dar_check::check(cbtc::dar_check::Params {
                ledger_host: ctx.ledger_host.clone(),
                access_token: ctx.access_token.clone(),
                dar_dirs: ctx.dar_dirs.clone(),
            })
            .await
            .map_err(AppError::Op)?;
            let rows = result
                .found
                .iter()
                .map(|p| {
                    ResultRow::new(
                        vec!["found".into(), p.name.clone(), p.version.clone()],
                        Some(format!("{p:#?}")),
                    )
                })
                .chain(result.missing.iter().map(|p| {
                    ResultRow::new(
                        vec!["MISSING".into(), p.name.clone(), p.version.clone()],
                        Some(format!("{p:#?}")),
                    )
                }))
                .collect();
            let status = match result.status {
                cbtc::dar_check::DarCheckStatus::Pass => "Pass",
                cbtc::dar_check::DarCheckStatus::Fail => "Fail",
            };
            Ok(OpResult::Table {
                title: format!(
                    "DARs {}/{} ({status})",
                    result.found.len(),
                    result.total_expected,
                ),
                columns: vec!["Status".into(), "Package".into(), "Version".into()],
                rows,
            })
        }
        Operation::Credentials => {
            let creds = cbtc::credentials::list_credentials(
                cbtc::credentials::ListCredentialsParams {
                    ledger_host: ctx.ledger_host.clone(),
                    party: ctx.party.clone(),
                    access_token: ctx.access_token.clone(),
                },
            )
            .await
            .map_err(AppError::Op)?;
            let rows = creds
                .iter()
                .map(|c| {
                    let claims = c
                        .claims
                        .iter()
                        .map(|cl| format!("{}={}", cl.property, cl.value))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ResultRow::new(
                        vec![c.id.clone(), c.description.clone(), claims],
                        Some(format!("{c:#?}")),
                    )
                })
                .collect();
            Ok(OpResult::Table {
                title: format!("Credentials ({})", creds.len()),
                columns: vec!["ID".into(), "Description".into(), "Claims".into()],
                rows,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbtc::DamlDecimal;
    use serde_json::json;
    use strum::IntoEnumIterator;

    #[test]
    fn operation_iter_and_labels() {
        // Act
        let ops: Vec<Operation> = Operation::iter().collect();
        // Assert
        assert_eq!(ops.len(), 8);
        assert_eq!(Operation::CheckBalance.to_string(), "Check Balance");
        assert_eq!(Operation::DepositAddresses.to_string(), "Deposit Addresses");
    }

    #[test]
    fn balance_formats_total_and_rows() {
        // Arrange
        let rows = vec![
            ("00aabbccddeeff".to_string(), DamlDecimal::parse("0.5").unwrap(), None),
            ("00112233445566".to_string(), DamlDecimal::parse("0.25").unwrap(), None),
        ];
        // Act
        let result = balance_to_result(&rows);
        // Assert
        match result {
            OpResult::Table { title, columns, rows } => {
                assert!(title.contains("0.75"));
                assert_eq!(columns, vec!["#", "Amount", "Contract"]);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].cells[1], "0.5");
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn balance_row_carries_detail() {
        // Arrange
        let rows = vec![(
            "00aabb".to_string(),
            DamlDecimal::parse("1").unwrap(),
            Some("{\"amount\":\"1\"}".to_string()),
        )];
        // Act
        let result = balance_to_result(&rows);
        // Assert
        match result {
            OpResult::Table { rows, .. } => {
                assert_eq!(rows[0].detail.as_deref(), Some("{\"amount\":\"1\"}"));
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn transfer_row_extracts_counterparty() {
        // Arrange
        let arg = json!({
            "transfer": {
                "sender": "bob::1220",
                "amount": "0.1",
                "requestedAt": "2026-01-01T00:00:00Z",
                "executeBefore": "2026-01-08T00:00:00Z"
            }
        });
        // Act
        let row = parse_transfer_row(&arg, "sender").unwrap();
        // Assert
        assert_eq!(row.counterparty, "bob::1220");
        assert_eq!(row.amount, "0.1");
        assert_eq!(row.execute_before, "2026-01-08T00:00:00Z");
    }
}
