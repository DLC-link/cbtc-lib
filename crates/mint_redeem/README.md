# CBTC Mint & Redeem

This crate provides functionality for minting CBTC from Bitcoin and redeeming CBTC back to Bitcoin through the Bitsafe Attestor network.

## Overview

CBTC (Canton Bitcoin) is a tokenized representation of Bitcoin on the Canton network. The minting and redemption process is secured by a decentralized attestor network that monitors Bitcoin transactions and confirms deposits/withdrawals.

### Minting Flow (BTC → CBTC)

1. **Create Deposit Account** - Create a Canton contract that will hold your pending deposits
2. **Get Bitcoin Address** - Request a unique Bitcoin address from the attestor network
3. **Send BTC** - Send Bitcoin to the provided address
4. **Attestor Confirmation** - The attestor network detects and confirms your deposit (requires 6+ confirmations)
5. **Deposit Request Created** - A DepositRequest contract is automatically created on Canton
6. **CBTC Minted** - CBTC tokens are minted to your Canton party

### Redemption Flow (CBTC → BTC)

1. **Create Withdraw Account** - Create a Canton contract with your destination Bitcoin address
2. **Burn CBTC** - Burn your CBTC holdings to create a withdraw request
3. **Attestor Processing** - The attestor network validates and processes your withdrawal
4. **BTC Sent** - Bitcoin is sent to your specified address

## Modules

- **`mint`** - Functions for creating deposit accounts and minting CBTC
- **`redeem`** - Functions for creating withdraw accounts and redeeming CBTC
- **`attestor`** - API client for the Bitsafe Attestor network
- **`models`** - Data structures for deposit/withdraw accounts and requests
- **`constants`** - Template IDs and choice names for Canton contracts

## Usage

### Prerequisites

Add the following to your `.env` file:

```env
# Canton Ledger
LEDGER_HOST=https://participant.example.com
PARTY_ID=your-party::1220...

# Keycloak Authentication
KEYCLOAK_HOST=https://keycloak.example.com
KEYCLOAK_REALM=your-realm
KEYCLOAK_CLIENT_ID=your-client-id
KEYCLOAK_USERNAME=your-username
KEYCLOAK_PASSWORD=your-password

# Bitsafe Attestor Network
ATTESTOR_URL=https://devnet.dlc.link/attestor-1
CANTON_NETWORK=canton-devnet

# Bitcoin address for withdrawals (optional - only needed for redeeming)
DESTINATION_BTC_ADDRESS=your-btc-address
```

### Example: Mint CBTC

```rust
use mint_redeem::{attestor, mint};
use keycloak::login::{password, password_url, PasswordParams};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Step 1: Authenticate with Keycloak to get an access token
    // This token is required for all Canton ledger operations
    let login_response = password(PasswordParams {
        client_id: "your-client-id".to_string(),
        username: "your-username".to_string(),
        password: "your-password".to_string(),
        url: password_url("https://keycloak.example.com", "your-realm"),
    }).await?;

    // Step 2: Get account contract rules from the attestor network
    // These rules define the template IDs and contract structures needed
    // to create deposit accounts that the attestor will recognize
    let account_rules = attestor::get_account_contract_rules(
        "https://devnet.dlc.link/attestor-1",
        "canton-devnet"
    ).await?;

    // Step 3: Create a deposit account on Canton
    // This creates a DepositAccount contract that will track your BTC deposits
    // The attestor network monitors these accounts and creates DepositRequests
    // when BTC arrives at the associated Bitcoin address
    let deposit_account = mint::create_deposit_account(
        mint::CreateDepositAccountParams {
            ledger_host: "https://participant.example.com".to_string(),
            party: "party::1220...".to_string(),
            user_name: "your-username".to_string(),
            access_token: login_response.access_token.clone(),
            account_rules,
        }
    ).await?;

    println!("Deposit account created: {}", deposit_account.id);

    // Step 4: Get the Bitcoin address for this deposit account
    // The attestor generates a unique BTC address for your deposit account
    // Any BTC sent to this address will be detected by the attestor network
    // and converted to CBTC after 6+ Bitcoin confirmations
    let bitcoin_address = mint::get_bitcoin_address(
        mint::GetBitcoinAddressParams {
            attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
            account_id: deposit_account.id.clone(),
            chain: "canton-devnet".to_string(),
        }
    ).await?;

    println!("Send BTC to: {}", bitcoin_address);
    println!("Waiting for BTC deposit (requires 6+ confirmations)...");

    // Step 5: Monitor for deposit requests
    // Once the attestor confirms your BTC deposit (6+ blocks), it automatically
    // creates a DepositRequest contract on Canton and mints CBTC to your party
    // We poll periodically to detect when the CBTC has been minted
    loop {
        let requests = mint::list_deposit_requests(
            mint::ListDepositRequestsParams {
                ledger_host: "https://participant.example.com".to_string(),
                party: "party::1220...".to_string(),
                access_token: login_response.access_token.clone(),
            }
        ).await?;

        if !requests.is_empty() {
            println!("CBTC minted! {} deposit request(s) found", requests.len());
            for request in &requests {
                println!("  Amount: {} BTC (tx: {})", request.amount, request.btc_tx_id);
            }
            break;
        }

        // Check every 30 seconds for new deposits
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }

    Ok(())
}
```

### Example: Redeem CBTC (Burn)

```rust
use mint_redeem::{attestor, redeem};
use keycloak::login::{password, password_url, PasswordParams};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Step 1: Authenticate with Keycloak
    let login_response = password(PasswordParams {
        client_id: "your-client-id".to_string(),
        username: "your-username".to_string(),
        password: "your-password".to_string(),
        url: password_url("https://keycloak.example.com", "your-realm"),
    }).await?;

    // Step 2: Get account contract rules from attestor
    // These rules are needed to create a WithdrawAccount that the attestor
    // will recognize and process for BTC withdrawals
    let account_rules = attestor::get_account_contract_rules(
        "https://devnet.dlc.link/attestor-1",
        "canton-devnet"
    ).await?;

    // Step 3: Create a withdraw account with your destination BTC address
    // This creates a WithdrawAccount contract that specifies where the BTC
    // should be sent when you burn CBTC. The destination address is locked
    // in the contract and cannot be changed after creation.
    let withdraw_account = redeem::create_withdraw_account(
        redeem::CreateWithdrawAccountParams {
            ledger_host: "https://participant.example.com".to_string(),
            party: "party::1220...".to_string(),
            user_name: "your-username".to_string(),
            access_token: login_response.access_token.clone(),
            account_rules_contract_id: account_rules.wa_rules.contract_id,
            account_rules_template_id: account_rules.wa_rules.template_id,
            account_rules_created_event_blob: account_rules.wa_rules.created_event_blob,
            destination_btc_address: "your-btc-address".to_string(),
        }
    ).await?;

    println!("Withdraw account created: {}", withdraw_account.contract_id);
    println!("BTC will be sent to: {}", withdraw_account.destination_btc_address);

    // Step 4: List your CBTC holdings to select which to burn
    // Holdings are UTXO-based, similar to Bitcoin. You may have multiple
    // holding contracts each with a different amount. We need to select
    // enough holdings to cover the burn amount.
    let holdings = redeem::list_holdings(
        redeem::ListHoldingsParams {
            ledger_host: "https://participant.example.com".to_string(),
            party: "party::1220...".to_string(),
            access_token: login_response.access_token.clone(),
        }
    ).await?;

    // Find CBTC holdings owned by your party
    let cbtc_holdings: Vec<_> = holdings
        .iter()
        .filter(|h| h.instrument_id == "CBTC" && h.owner == "party::1220...")
        .collect();

    if cbtc_holdings.is_empty() {
        return Err("No CBTC holdings found to burn".to_string());
    }

    // Step 5: Select holdings to burn
    // Select enough holdings to cover the burn amount. Similar to Bitcoin
    // UTXO selection, we may need to combine multiple holdings.
    let burn_amount = "0.001"; // Amount of BTC to withdraw
    let burn_amount_f64: f64 = burn_amount.parse().unwrap();

    let mut selected_holdings = Vec::new();
    let mut selected_total = 0.0;

    for holding in &cbtc_holdings {
        let amount = holding.amount.parse::<f64>().unwrap_or(0.0);
        selected_holdings.push(holding.contract_id.clone());
        selected_total += amount;
        if selected_total >= burn_amount_f64 {
            break;
        }
    }

    if selected_total < burn_amount_f64 {
        return Err(format!("Insufficient balance. Have {}, need {}", selected_total, burn_amount));
    }

    // Step 6: Get token standard contracts from attestor
    // These contracts (burn_mint_factory, instrument_configuration) are required
    // by the Canton token standard to properly burn CBTC and create a withdrawal
    let token_contracts = attestor::get_token_standard_contracts(
        "https://devnet.dlc.link/attestor-1",
        "canton-devnet"
    ).await?;

    // Step 7: Request withdrawal by burning CBTC
    // This burns the selected CBTC holdings and creates a WithdrawRequest
    // contract on Canton. The attestor network detects this request and
    // sends BTC to your destination address.
    let withdraw_request = redeem::request_withdraw(
        redeem::RequestWithdrawParams {
            ledger_host: "https://participant.example.com".to_string(),
            party: "party::1220...".to_string(),
            user_name: "your-username".to_string(),
            access_token: login_response.access_token.clone(),
            attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
            chain: "canton-devnet".to_string(),
            withdraw_account_contract_id: withdraw_account.contract_id,
            amount: burn_amount.to_string(),
            holding_contract_ids: selected_holdings,
        }
    ).await?;

    println!("Withdrawal requested!");
    println!("CBTC burned: {}", burn_amount);
    println!("BTC will be sent to: {}", withdraw_account.destination_btc_address);

    // Step 8: Monitor withdrawal status
    // The attestor processes withdrawals and updates the btc_tx_id field
    // when the BTC transaction is broadcast
    loop {
        let requests = redeem::list_withdraw_requests(
            redeem::ListWithdrawRequestsParams {
                ledger_host: "https://participant.example.com".to_string(),
                party: "party::1220...".to_string(),
                access_token: login_response.access_token.clone(),
            }
        ).await?;

        if let Some(request) = requests.iter().find(|r| r.contract_id == withdraw_request.contract_id) {
            if let Some(tx_id) = &request.btc_tx_id {
                println!("BTC sent! Transaction ID: {}", tx_id);
                break;
            }
        }

        println!("Waiting for attestor to process withdrawal...");
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }

    Ok(())
}
```

### Example: Check Account Status

The `get_deposit_account_status()` function provides a unified view of your deposit account by combining data from both the Canton ledger and the attestor network. This is useful for:

- **Checking if deposits are pending**: See if BTC has been detected but not yet confirmed
- **Getting your Bitcoin address**: Retrieve the BTC address without making a separate attestor API call
- **Monitoring account state**: Track the last processed Bitcoin block height

```rust
use mint_redeem::mint;

// Get comprehensive status for a deposit account
// This combines Canton ledger data (owner, contract_id) with attestor data
// (bitcoin_address, pending_balance) to give you a complete view
let status = mint::get_deposit_account_status(
    mint::GetDepositAccountStatusParams {
        ledger_host: "https://participant.example.com".to_string(),
        party: "party::1220...".to_string(),
        access_token: access_token.clone(),
        attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
        chain: "canton-devnet".to_string(),
        account_id: "your-account-id".to_string(),
    }
).await?;

// The Bitcoin address where you should send BTC
println!("Bitcoin Address: {}", status.bitcoin_address);

// Pending balance indicates BTC detected but not yet confirmed (< 6 confirmations)
// Once confirmed, this returns to "0" and a DepositRequest is created
println!("Pending Balance: {} BTC", status.pending_balance);
println!("Has Pending: {}", status.has_pending_balance);

// The last Bitcoin block height that was scanned for deposits
// Useful for debugging if deposits aren't being detected
println!("Last Processed Block: {}", status.last_processed_bitcoin_block);
```

### Running the Complete Examples

Complete end-to-end examples are available:

```bash
# Copy and configure your environment
cp .env.example .env
# Edit .env with your values

# Run the mint flow example (BTC → CBTC)
cargo run --example mint_cbtc_flow

# Run the redeem flow example (CBTC → BTC)
cargo run --example redeem_cbtc_flow

# Monitor deposits in real-time
cargo run --example monitor_deposits
```

## API Reference

### Mint Module

#### `list_deposit_accounts()`

List all deposit accounts for a party.

#### `create_deposit_account()`

Create a new deposit account that can receive BTC deposits.

#### `get_bitcoin_address()`

Get the Bitcoin address for a deposit account from the attestor network.

#### `list_deposit_requests()`

List all deposit requests (completed deposits that have been confirmed and minted).

#### `get_deposit_account_status()`

Get the full status of a deposit account including Bitcoin address, pending balance, and last processed block height. This combines data from both Canton and the attestor network.

#### `mint_cbtc()`

Manually mint CBTC from a confirmed deposit request. Usually this happens automatically, but this function allows manual triggering if needed.

### Redeem Module

#### `list_withdraw_accounts()`

List all withdraw accounts for a party.

#### `create_withdraw_account()`

Create a new withdraw account with a destination Bitcoin address for redemptions.

#### `list_holdings()`

List all CBTC holdings (unlocked UTXOs) available for burning.

#### `request_withdraw()`

Burn CBTC holdings and create a withdraw request that the attestor will process.

#### `list_withdraw_requests()`

List all withdraw requests and their status (including BTC transaction IDs once processed).

### Attestor Module

#### `get_account_contract_rules()`

Get the DepositAccountRules and WithdrawAccountRules contracts from the attestor. Required for creating accounts.

#### `get_bitcoin_address()`

Get the Bitcoin address for a specific account ID.

#### `get_token_standard_contracts()`

Get the token standard contracts (burn_mint_factory, instrument_configuration, etc.). Required for burning CBTC.

## Testing

Run the attestor API tests:

```bash
cargo test --package mint_redeem
```

Make sure your `.env` file is configured with valid credentials before running tests.

## Environments

### Devnet

- Attestor: `https://devnet.dlc.link/attestor-1`
- Network: `canton-devnet`
- Bitcoin: Regtest or Bitcoin testnet

### Testnet

- Attestor: `https://testnet.dlc.link/attestor-1`
- Network: `canton-testnet`
- Bitcoin: Bitcoin testnet

### Mainnet

- Attestor: `https://dlc.link/attestor-1`
- Network: `canton-mainnet`
- Bitcoin: Bitcoin mainnet

## Important Notes

1. **Bitcoin Monitoring**: This library does NOT monitor Bitcoin transactions. The attestor network handles that. You simply need to:

   - Get a Bitcoin address from the attestor
   - Send BTC to that address
   - Wait for attestor confirmation (6+ blocks)
   - Check for DepositRequests on Canton

2. **Pending Balance**: When BTC is detected but not yet fully confirmed (< 6 blocks), the deposit account's `pending_balance` field will be non-zero and `has_pending_balance` will be true. Once confirmed (6+ blocks), the balance returns to "0" and a DepositRequest is created with CBTC minted.

3. **Multiple Deposits**: You can send multiple BTC deposits to the same Bitcoin address. Each confirmed deposit will create a separate DepositRequest and mint CBTC independently.

4. **UTXO Model**: CBTC uses a UTXO model similar to Bitcoin. When burning CBTC, you must select specific holdings (UTXOs) to burn, and any excess will be returned as change in a new holding.

5. **Locked Holdings**: Holdings may be temporarily locked during transfer or other operations. The `list_holdings()` function automatically filters out locked holdings, returning only those available for burning.

6. **Destination Address**: The destination Bitcoin address for a WithdrawAccount is locked at creation time and cannot be changed. Create a new WithdrawAccount if you need to withdraw to a different address.

7. **Minimum Amounts**: Check with your attestor network for minimum deposit/withdrawal amounts and any associated fees.

## License

MIT
