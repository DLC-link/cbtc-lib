# CBTC Mint & Redeem

This crate provides functionality for minting CBTC from Bitcoin and redeeming CBTC back to Bitcoin through the Bitsafe Attestor network.

## Overview

CBTC (Canton Bitcoin) is a tokenized representation of Bitcoin on the Canton network. The minting and redemption process is secured by a decentralized attestor network that monitors Bitcoin transactions and confirms deposits/withdrawals.

### Minting Flow (BTC → CBTC)

1. **Create Deposit Account** - Create a Canton contract that will hold your pending deposits
2. **Get Bitcoin Address** - Request a unique Bitcoin address from the attestor network
3. **Send BTC** - Send Bitcoin to the provided address
4. **Attestor Confirmation** - The attestor network detects and confirms your deposit
5. **Deposit Request Created** - A DepositRequest contract is automatically created on Canton
6. **CBTC Minted** - CBTC is minted to your Canton party

### Redemption Flow (CBTC → BTC)

1. **Create Withdraw Account** - Create a Canton contract for withdrawals
2. **Burn CBTC** - Burn your CBTC holdings to create a withdraw request
3. **Attestor Processing** - The attestor network processes your withdrawal
4. **BTC Sent** - Bitcoin is sent to your specified address

## Modules

- **`mint`** - Functions for creating deposit accounts and minting CBTC
- **`redeem`** - Functions for creating withdraw accounts and redeeming CBTC (coming soon)
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
```

### Example: Mint CBTC

```rust
use mint_redeem::{attestor, mint};
use keycloak::login::{password, password_url, PasswordParams};

#[tokio::main]
async fn main() -> Result<(), String> {
    // 1. Authenticate
    let login_response = password(PasswordParams {
        client_id: "your-client-id".to_string(),
        username: "your-username".to_string(),
        password: "your-password".to_string(),
        url: password_url("https://keycloak.example.com", "your-realm"),
    }).await?;

    // 2. Get account rules from attestor
    let account_rules = attestor::get_account_contract_rules(
        "https://devnet.dlc.link/attestor-1",
        "canton-devnet"
    ).await?;

    // 3. Create a deposit account
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

    // 4. Get Bitcoin address
    let bitcoin_address = mint::get_bitcoin_address(
        mint::GetBitcoinAddressParams {
            attestor_url: "https://devnet.dlc.link/attestor-1".to_string(),
            account_id: deposit_account.id.clone(),
            chain: "canton-devnet".to_string(),
        }
    ).await?;

    println!("Send BTC to: {}", bitcoin_address);

    // 5. Monitor for deposit requests
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
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }

    Ok(())
}
```

### Example: Check Account Status

```rust
use mint_redeem::mint;

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

println!("Bitcoin Address: {}", status.bitcoin_address);
println!("Pending Balance: {}", status.pending_balance);
println!("Has Pending: {}", status.has_pending_balance);
```

### Running the Complete Example

A complete end-to-end example is available:

```bash
# Copy and configure your environment
cp .env.example .env
# Edit .env with your values

# Run the mint flow example
cargo run --example mint_cbtc_flow
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
List all deposit requests (completed deposits that have been confirmed).

#### `get_deposit_account_status()`
Get the full status of a deposit account including Bitcoin address and pending balance.

### Attestor Module

#### `get_account_contract_rules()`
Get the DepositAccountRules and WithdrawAccountRules contracts from the attestor.

#### `get_bitcoin_address()`
Get the Bitcoin address for a specific account ID.

#### `get_token_standard_contracts()`
Get the token standard contracts (burn_mint_factory, instrument_configuration, etc.).

## Data Structures

### DepositAccount

Represents a deposit account contract on Canton.

```rust
pub struct DepositAccount {
    pub contract_id: String,              // Canton contract ID
    pub id: String,                       // Unique account UUID
    pub owner: String,                    // Canton party ID of owner
    pub pending_balance: String,          // BTC pending confirmation (as string)
    pub last_processed_bitcoin_block: i64, // Last BTC block processed
}
```

### DepositRequest

Represents a confirmed deposit that resulted in CBTC being minted.

```rust
pub struct DepositRequest {
    pub contract_id: String,         // Canton contract ID
    pub deposit_account_id: String,  // Associated deposit account
    pub amount: String,              // Amount of BTC deposited
    pub btc_tx_id: String,          // Bitcoin transaction ID
}
```

### DepositAccountStatus

Full status information for a deposit account.

```rust
pub struct DepositAccountStatus {
    pub contract_id: String,
    pub id: String,
    pub owner: String,
    pub has_pending_balance: bool,        // True if deposit is processing
    pub pending_balance: String,
    pub bitcoin_address: String,          // BTC address for deposits
    pub last_processed_bitcoin_block: i64,
}
```

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

### Testnet
- Attestor: `https://testnet.dlc.link/attestor-1`
- Network: `canton-testnet`

### Mainnet
- Attestor: `https://dlc.link/attestor-1`
- Network: `canton-mainnet`

## Important Notes

1. **Bitcoin Monitoring**: This library does NOT monitor Bitcoin transactions. The attestor network handles that. You simply need to:
   - Get a Bitcoin address from the attestor
   - Send BTC to that address
   - Wait for attestor confirmation
   - Check for DepositRequests on Canton

2. **Pending Balance**: When BTC is detected but not yet fully confirmed, the deposit account's `pending_balance` field will be non-zero. Once confirmed, the balance returns to 0 and a DepositRequest is created.

3. **Multiple Deposits**: You can send multiple BTC deposits to the same Bitcoin address. Each confirmed deposit will create a separate DepositRequest.

4. **Minimum Amounts**: Check with your attestor network for minimum deposit/withdrawal amounts.

## License

MIT
