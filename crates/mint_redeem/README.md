# CBTC Mint & Redeem

This crate provides functionality for minting CBTC from Bitcoin and redeeming CBTC back to Bitcoin through the Bitsafe Attestor network.

## Overview

CBTC (Canton Bitcoin) is a tokenized representation of Bitcoin on the Canton network. The minting and redemption process is secured by a decentralized attestor network that monitors Bitcoin transactions and confirms deposits/withdrawals.

### Minting Flow (BTC → CBTC)

1. **Create Deposit Account** - Create a Canton contract for receiving deposits
2. **Get Bitcoin Address** - Request a unique Bitcoin address from the attestor network
3. **Send BTC** - Send Bitcoin to the provided address (external to this library)
4. **Attestor Monitors** - The attestor network detects and confirms your deposit (requires 6+ confirmations)
5. **CBTC Minted** - After confirmation, attestors automatically mint CBTC tokens directly to your Canton party

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

### Example: Set Up Deposit Account for Minting

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
    // This creates a DepositAccount contract for receiving BTC deposits
    let deposit_account = mint::create_deposit_account(
        mint::CreateDepositAccountParams {
            ledger_host: "https://participant.example.com".to_string(),
            party: "party::1220...".to_string(),
            user_name: "your-username".to_string(),
            access_token: login_response.access_token.clone(),
            account_rules,
        }
    ).await?;

    log::debug!("Deposit account created: {}", deposit_account.contract_id);

    // Step 4: Get the Bitcoin address for this deposit account
    // The attestor generates a unique BTC address for your deposit account
    // Any BTC sent to this address will be detected by the attestor network
    // and converted to CBTC after 6+ Bitcoin confirmations
    let bitcoin_address = mint::get_bitcoin_address(
        mint::GetBitcoinAddressParams {
            attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
            account_contract_id: deposit_account.contract_id.clone(),
            chain: "canton-devnet".to_string(),
        }
    ).await?;

    log::debug!("Send BTC to: {}", bitcoin_address);
    log::debug!("After 6+ confirmations, attestors will automatically mint CBTC to your party");

    // That's it! The attestor network handles everything else:
    // - Monitors Bitcoin for deposits to this address
    // - Waits for 6+ confirmations
    // - Automatically mints CBTC tokens to your party
    //
    // To monitor for minted CBTC, you can periodically run:
    // cargo run -p examples --bin check_balance
    //
    // Or programmatically check your holdings:
    // redeem::list_holdings() will show your CBTC balance

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

    log::debug!("Withdraw account created: {}", withdraw_account.contract_id);
    log::debug!("BTC will be sent to: {}", withdraw_account.destination_btc_address);

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

    log::debug!("Withdrawal requested!");
    log::debug!("CBTC burned: {}", burn_amount);
    log::debug!("BTC will be sent to: {}", withdraw_account.destination_btc_address);

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
                log::debug!("BTC sent! Transaction ID: {}", tx_id);
                break;
            }
        }

        log::debug!("Waiting for attestor to process withdrawal...");
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }

    Ok(())
}
```

### Example: Check Deposit Account Status

The `get_deposit_account_status()` function provides a view of your deposit account by combining data from both the Canton ledger and the attestor network:

```rust
use mint_redeem::mint;

// Get status for a deposit account
// This combines Canton ledger data (owner, contract_id, last processed block)
// with attestor data (bitcoin_address)
let status = mint::get_deposit_account_status(
    mint::GetDepositAccountStatusParams {
        ledger_host: "https://participant.example.com".to_string(),
        party: "party::1220...".to_string(),
        access_token: access_token.clone(),
        attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
        chain: "canton-devnet".to_string(),
        account_contract_id: "your-account-contract-id".to_string(),
    }
).await?;

// The Bitcoin address where you should send BTC
log::debug!("Bitcoin Address: {}", status.bitcoin_address);

// The last Bitcoin block height that was scanned by attestors
log::debug!("Last Processed Block: {}", status.last_processed_bitcoin_block);

// Contract details
log::debug!("Owner: {}", status.owner);
log::debug!("Contract ID: {}", status.contract_id);
```

### Running Tests

Run the mint_redeem crate tests:

```bash
# Copy and configure your environment
cp .env.example .env
# Edit .env with your values

# Run all tests for mint_redeem crate
cargo test --package mint_redeem
```

## API Reference

### Mint Module

#### `list_deposit_accounts()`

List all deposit accounts for a party.

#### `create_deposit_account()`

Create a new deposit account that can receive BTC deposits.

#### `get_bitcoin_address()`

Get the Bitcoin address for a deposit account from the attestor network.

#### `get_deposit_account_status()`

Get the full status of a deposit account including Bitcoin address and last processed block height. This combines data from both Canton and the attestor network.

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
   - Attestors automatically mint CBTC to your party

2. **No DepositRequest Contracts**: There are no DepositRequest contracts. After 6+ Bitcoin confirmations, attestors directly mint CBTC tokens (as Holding contracts) to your party.

3. **Multiple Deposits**: You can send multiple BTC deposits to the same Bitcoin address. Each confirmed deposit will mint CBTC independently.

4. **UTXO Model**: CBTC uses a UTXO model similar to Bitcoin. When burning CBTC, you must select specific holdings (UTXOs) to burn, and any excess will be returned as change in a new holding.

5. **Locked Holdings**: Holdings may be temporarily locked during transfer or other operations. The `list_holdings()` function automatically filters out locked holdings, returning only those available for burning.

6. **Destination Address**: The destination Bitcoin address for a WithdrawAccount is locked at creation time and cannot be changed. Create a new WithdrawAccount if you need to withdraw to a different address.

7. **Minimum Amounts**: Check with your attestor network for minimum deposit/withdrawal amounts and any associated fees.

## License

MIT
