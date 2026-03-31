# Examples

This directory contains example programs demonstrating how to use the Canton CBTC library.

## Setup

1. Copy `.env.example` to `.env` in the project root:

   ```bash
   cp .env.example .env
   ```

2. Fill in your configuration values in `.env`

## Running Examples

All examples should be run from the project root directory using `cargo run --example <name>`.

### Prerequisites for Mint & Redeem Examples

The mint/redeem examples require a Minter credential issued by the CBTC registrar. Run examples in this order:

1. **`credentials`** — Check for existing credentials, accept a pending Minter credential offer if needed
2. **`mint_cbtc_flow`** — Create a deposit account and get a BTC address (requires Minter credential)
3. **`redeem_cbtc_flow`** — Create a withdraw account and burn CBTC (requires Minter credential + CBTC balance)

These also require `BITSAFE_API_URL` in your `.env`.

### Integration Test

End-to-end CBTC flow covering credential validation, account setup, transfers, withdrawals, and UTXO consolidation:

```bash
cargo run --example integration_test
```

Requires two parties (sender + receiver) with separate Keycloak credentials. The sender must have a Minter credential and a non-zero CBTC balance. See the [Environment Variables](#environment-variables) section for required config.

Additional env vars for this example:

- `BITSAFE_API_URL` (required) — Bitsafe API base URL
- `RECEIVER_KEYCLOAK_USERNAME`, `RECEIVER_KEYCLOAK_PASSWORD`, `RECEIVER_KEYCLOAK_CLIENT_ID`, `RECEIVER_PARTY_ID` (required)
- `RECEIVER_LEDGER_HOST`, `RECEIVER_KEYCLOAK_HOST`, `RECEIVER_KEYCLOAK_REALM` (optional, falls back to sender values)
- `DESTINATION_BTC_ADDRESS` (optional, default: testnet address)
- `WITHDRAW_AMOUNT` (optional, default: `TRANSFER_AMOUNT`)
- `FAUCET_URL` (optional, enables faucet steps)
- `FAUCET_NETWORK` (optional, default: `"devnet"`)

#### Test Steps (Given/When/Then)

[**Step 1: Check sender balance**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L249)
- **Given** the sender party is configured with valid Keycloak credentials
- **When** fetching active holding contracts for the sender
- **Then** the sender has a positive CBTC balance (fails if zero)

[**Step 2: Check receiver balance**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L258)
- **Given** the receiver party is configured with valid Keycloak credentials
- **When** fetching active holding contracts for the receiver
- **Then** the receiver's current balance is reported (may be zero)

[**Step 3: Fetch Minter credentials**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L264)
- **Given** the sender has Minter credentials on Canton
- **When** listing credentials and filtering for `hasCBTCRole == "Minter"`
- **Then** at least one Minter credential contract ID is found

[**Step 4: Fetch account rules**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L290)
- **When** calling `get_account_contract_rules` on the Bitsafe API
- **Then** an `AccountContractRuleSet` with deposit and withdraw rules is returned

[**Step 5: Create deposit account**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L298)
- **Given** credential CIDs from step 3 and account rules from step 4
- **When** exercising the `CreateDepositAccount` choice on Canton
- **Then** a new `DepositAccount` contract is created with `owner == sender.party_id`

[**Step 6: Get Bitcoin address for deposit account**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L323)
- **Given** the deposit account from step 5
- **When** calling `get_bitcoin_address` via the Bitsafe API
- **Then** a valid BTC address is returned for the account

[**Step 7: Create withdraw account**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L336)
- **Given** credential CIDs from step 3, withdraw account rules from step 4, and a destination BTC address
- **When** exercising the `CreateWithdrawAccount` choice on Canton
- **Then** a new `WithdrawAccount` contract is created with the specified destination address

**Steps 8-10: Faucet deposit** *(only if `FAUCET_URL` is set)*

[**Step 8: Request CBTC from faucet**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L362)
- **Given** a baseline incoming transfer count captured before the request
- **When** POSTing to `{FAUCET_URL}/api/faucet` with sender's party ID and amount
- **Then** the faucet returns `success: true` and submits a CBTC transfer to the sender

[**Step 9: Poll for incoming faucet transfer**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L411)
- **Given** a faucet request submitted in step 8
- **When** polling `fetch_incoming_transfers` every 3s for up to 30s
- **Then** the incoming transfer count exceeds the pre-faucet baseline

[**Step 10: Accept faucet transfer**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L439)
- **Given** an incoming faucet transfer detected in step 9
- **When** calling `accept_all` for the sender party
- **Then** the faucet transfer is accepted with zero failures

[**Step 11: Send CBTC to receiver**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L462)
- **Given** the sender has a CBTC balance
- **When** submitting a transfer of `TRANSFER_AMOUNT` CBTC from sender to receiver
- **Then** a transfer offer is created on Canton

[**Step 12: List outgoing offers (sender)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L492)
- **Given** a transfer was submitted in step 11
- **When** fetching the sender's outgoing transfer offers
- **Then** at least one pending offer exists

[**Step 13: List incoming offers (receiver)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L507)
- **Given** a transfer offer exists from step 11
- **When** fetching the receiver's incoming transfer offers
- **Then** at least one pending offer exists

[**Step 14: Accept transfers (receiver)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L522)
- **Given** the receiver has pending incoming offers
- **When** calling `accept_all` for the receiver party
- **Then** all offers are accepted with zero failures

[**Step 15: Check receiver balance**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L542)
- **Given** the receiver accepted a transfer in step 14
- **When** fetching the receiver's balance
- **Then** the balance reflects the received amount

[**Step 16: Return CBTC to sender**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L548)
- **Given** the receiver has CBTC from step 14
- **When** submitting a transfer of `TRANSFER_AMOUNT` CBTC from receiver back to sender
- **Then** a return transfer offer is created on Canton

[**Step 17: Accept transfers (sender)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L578)
- **Given** the sender has a pending incoming offer from step 16
- **When** calling `accept_all` for the sender party
- **Then** all offers are accepted with zero failures

[**Step 18: Check sender balance (pre-withdraw)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L598)
- **Given** the sender accepted the return transfer in step 17
- **When** fetching the sender's balance
- **Then** the balance is captured as the pre-withdraw baseline

[**Step 19: Submit withdrawal**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L605)
- **Given** the withdraw account from step 7 and the sender has CBTC holdings
- **When** selecting holdings to cover `WITHDRAW_AMOUNT`, checking limits, and calling `submit_withdraw`
- **Then** the specified amount of CBTC is burned and the withdraw account's pending balance increases

[**Step 20: Check balance (post-withdraw)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L669)
- **Given** the sender's pre-withdraw balance from step 18
- **When** fetching the sender's balance after the withdrawal
- **Then** the balance has decreased (confirming tokens were burned)

[**Step 21: Consolidate UTXOs (sender)**](https://github.com/DLC-link/cbtc-lib/blob/09d85946e6ff23ac807813e465f767569adda971/examples/integration_test.rs#L685)
- **Given** the sender may have accumulated multiple UTXOs during the test
- **When** checking the UTXO count against `CONSOLIDATION_THRESHOLD`
- **Then** UTXOs are consolidated if the count exceeds the threshold, otherwise skipped

> **Note:** Step numbers above use faucet-enabled numbering (21 steps). Without `FAUCET_URL`, steps 8-10 are skipped and the total is 18 steps (the remaining steps shift down by 3).

> **Note:** Deposit and withdraw accounts created during the test are persistent Canton contracts. No cleanup API exists; they remain after the test.

### Credentials

List, accept, and manage CBTC Minter credentials:

```bash
cargo run --example credentials
```

This example checks for existing Minter credentials. If none are found, it looks for pending credential offers from the registrar, accepts the first Minter offer, and displays the credential CID for use in other operations.

### Mint CBTC Flow

Complete flow for minting CBTC from BTC:

```bash
cargo run --example mint_cbtc_flow
```

Creates a deposit account with Minter credentials, retrieves the BTC address, and displays account status. Requires a Minter credential (run `credentials` first).

### Redeem CBTC Flow

Complete flow for redeeming CBTC back to BTC:

```bash
cargo run --example redeem_cbtc_flow
```

Creates a withdraw account, checks transaction limits, and submits a withdrawal. Requires a Minter credential and CBTC balance.

### Test Burn CBTC

Burn a small amount of CBTC using an existing withdraw account:

```bash
cargo run --example test_burn_cbtc
```

### Check Balance

Check your CBTC balance and UTXO count:

```bash
cargo run --example check_balance
```

### Send CBTC

Send CBTC to another party:

```bash
# Set the amount and receiver in .env or environment
export TRANSFER_AMOUNT=0.1
export LIB_TEST_RECEIVER_PARTY_ID="receiver-party::1220..."
cargo run --example send_cbtc
```

### List Incoming Offers

List all pending CBTC transfer offers where you are the receiver:

```bash
cargo run --example list_incoming_offers
```

This example lists all pending transfers waiting for you to accept.

### List Outgoing Offers

List all pending CBTC transfer offers where you are the sender:

```bash
cargo run --example list_outgoing_offers
```

This example shows all transfers you've sent that haven't been accepted yet.

### Accept Pending Transfers

Accept all pending CBTC transfers for your party:

```bash
cargo run --example accept_transfers
```

This example automatically fetches all pending TransferInstruction contracts and accepts them in a loop. Useful for automated acceptance of incoming transfers.

### Cancel Pending Transfers

Cancel all pending outgoing transfers that haven't been accepted:

```bash
cargo run --example cancel_offers
```

This example withdraws all transfer offers you've sent that are still pending, returning the CBTC to your account.

### Stream CBTC

Stream CBTC to a single receiver multiple times:

```bash
# Set the streaming parameters
export RECEIVER_PARTY="receiver-party::1220..."
export TRANSFER_COUNT=10
export TRANSFER_AMOUNT=0.001
cargo run --example stream_cbtc
```

This example sends multiple transfers to the same receiver, useful for streaming payments or testing repeated transfers.

### Consolidate UTXOs

Check and consolidate UTXOs if needed:

```bash
# Optional: set custom threshold (default is 10)
export CONSOLIDATION_THRESHOLD=8
cargo run --example consolidate_utxos
```

### Batch Distribute

Distribute CBTC to multiple recipients from a CSV file:

```bash
# Create recipients.csv with format:
# receiver,amount
# party1::1220...,5.0
# party2::1220...,3.5

# Run the example
cargo run --example batch_distribute

# Or specify a custom CSV path
export RECIPIENTS_CSV=my_recipients.csv
cargo run --example batch_distribute
```

See `recipients_example.csv` for the CSV format.

### Batch Distribute with Callback

Distribute CBTC to multiple recipients with real-time result logging:

```bash
cargo run --example batch_with_callback
```

This example demonstrates the callback feature, which allows you to process transfer results as they complete. The callback writes one line per transfer to a timestamped log file.

## Transfer Result Callbacks

The batch distribution system supports optional callbacks that are invoked after each transfer completes (whether successful or failed). This allows for real-time processing of transfer results.

### TransferResult Structure

Each callback receives a `TransferResult` with the following fields:

```rust
pub struct TransferResult {
    pub success: bool,              // Whether the transfer succeeded
    pub transfer_index: usize,      // Index in the batch (0-based)
    pub receiver: String,           // Recipient address
    pub amount: String,             // Transfer amount
    pub transfer_offer_cid: Option<String>,  // Contract ID if successful
    pub update_id: Option<String>,  // Canton ledger update ID if successful
    pub reference: Option<String>,  // Unique reference ID (base64 encoded)
    pub raw_response: Option<String>, // Full JSON response from ledger
    pub error: Option<String>,      // Error message if failed
}
```

### Use Cases

1. **Real-time Logging**: Write results to a file or logging service as they complete
2. **Database Updates**: Store transfer records in a database immediately
3. **Progress Tracking**: Update UI or monitoring dashboards
4. **Notifications**: Send alerts for failed transfers
5. **Custom Retry Logic**: Implement application-specific retry strategies

### Basic Usage

```rust
use std::pin::Pin;
use std::future::Future;

let callback = Box::new(|result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        if result.success {
            println!("✅ Transfer succeeded: {} to {}", result.amount, result.receiver);
            // Log to database, send notification, etc.
        } else {
            println!("❌ Transfer failed: {}", result.error.unwrap_or_default());
            // Handle failure (retry, alert, etc.)
        }
    })
}) as Box<cbtc::transfer::TransferResultCallback>;

let result = cbtc::distribute::submit(cbtc::distribute::Params {
    // ... other params
    on_transfer_complete: Some(callback),
})
.await?;
```

### File Logging Example (One Line Per Event)

```rust
use std::fs::OpenOptions;
use std::io::Write;

let log_file = format!("transfer_results_{}.log", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

let callback = Box::new(move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let log_file = log_file.clone();
    Box::pin(async move {
        let status = if result.success { "SUCCESS" } else { "FAILED" };
        let reference = result.reference.as_deref().unwrap_or("N/A");
        let offer_cid = result.transfer_offer_cid.as_deref().unwrap_or("N/A");
        let update_id = result.update_id.as_deref().unwrap_or("N/A");
        let error = result.error.as_deref().unwrap_or("N/A");

        let log_line = format!(
            "{} | {} | idx={} | to={} | amount={} | ref={} | offer={} | update_id={} | error={}\n",
            chrono::Utc::now().to_rfc3339(),
            status,
            result.transfer_index,
            result.receiver,
            result.amount,
            reference,
            offer_cid,
            update_id,
            error
        );

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            let _ = file.write_all(log_line.as_bytes());
        }
    })
}) as Box<cbtc::transfer::TransferResultCallback>;
```

Example output:

```
2025-01-10T15:23:45.123Z | SUCCESS | idx=0 | to=merchant::1220... | amount=0.001 | ref=YmF0Y2gtMTIz... | offer=00123... | update_id=12208abc... | error=N/A
2025-01-10T15:23:46.456Z | FAILED | idx=1 | to=validator::5678... | amount=0.002 | ref=YmF0Y2gtNDU2... | offer=N/A | update_id=N/A | error=Insufficient funds
```

### Database Logging Example

```rust
let db_pool = /* your DB connection pool */;

let callback = Box::new(move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let pool = db_pool.clone();
    Box::pin(async move {
        sqlx::query!(
            "INSERT INTO transfer_logs (receiver, amount, success, reference, raw_response, error)
             VALUES ($1, $2, $3, $4, $5, $6)",
            result.receiver,
            result.amount,
            result.success,
            result.reference,
            result.raw_response,
            result.error
        )
        .execute(&pool)
        .await
        .unwrap();
    })
}) as Box<cbtc::transfer::TransferResultCallback>;
```

### Progress Tracking Example

```rust
use std::sync::{Arc, Mutex};

let progress = Arc::new(Mutex::new(0));
let total = recipients.len();

let callback = Box::new(move |result: cbtc::transfer::TransferResult| -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let progress = progress.clone();
    Box::pin(async move {
        let mut count = progress.lock().unwrap();
        *count += 1;
        println!("Progress: {}/{}", count, total);

        if result.success {
            println!("  ✓ {} transferred", result.amount);
        } else {
            println!("  ✗ Failed: {}", result.error.unwrap_or_default());
        }
    })
}) as Box<cbtc::transfer::TransferResultCallback>;
```

### No Callback (Default)

If you don't need callbacks, simply pass `None`:

```rust
let result = cbtc::distribute::submit(cbtc::distribute::Params {
    // ... other params
    on_transfer_complete: None,  // No callback
})
.await?;
```

### Important Notes

1. **Callbacks are async**: They can perform async operations like database writes
2. **Callbacks are called sequentially**: One completes before the next transfer starts
3. **Errors in callbacks don't stop the batch**: If a callback panics, it's isolated
4. **The reference field** contains the base64-encoded unique ID: `base64(reference_base + sender + receiver)`
5. **The update_id field** contains the Canton ledger's unique update ID for successful transfers (used for tracking and idempotency)
6. **The raw_response field** contains the full JSON response from the Canton ledger (for successful submissions)

See `batch_with_callback.rs` for a complete working example.

## Environment Variables

Required for all examples:

- `KEYCLOAK_HOST` - Your Keycloak host
- `KEYCLOAK_REALM` - Your Keycloak realm
- `KEYCLOAK_CLIENT_ID` - Client ID
- `KEYCLOAK_USERNAME` - Username
- `KEYCLOAK_PASSWORD` - Password
- `LEDGER_HOST` - Canton participant node URL
- `PARTY_ID` - Your party ID
- `DECENTRALIZED_PARTY_ID` - CBTC decentralized party ID
- `REGISTRY_URL` - Canton registry URL

Optional:

- `TRANSFER_AMOUNT` - Amount to send (default: 0.1)
- `LIB_TEST_RECEIVER_PARTY_ID` - Receiver party for transfers
- `CONSOLIDATION_THRESHOLD` - UTXO threshold for consolidation (default: 10)
- `RECIPIENTS_CSV` - Path to CSV file for batch distribution (default: recipients.csv)

For stream example:

- `RECEIVER_PARTY` - The party ID to receive all stream transfers
- `TRANSFER_COUNT` - Number of transfers to send in the stream
- `TRANSFER_AMOUNT` - Amount per transfer
