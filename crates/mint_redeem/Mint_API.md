# CBTC Mint API Flow

This document details the underlying HTTP APIs and Canton contracts used to set up deposit accounts for CBTC minting. This is intended for developers implementing the mint flow in other languages or debugging issues.

## Overview

This library provides functions to set up deposit accounts:

1. Authenticating with Keycloak to get a JWT access token
2. Getting account contract rules from the attestor network
3. Creating a DepositAccount contract on Canton
4. Getting a Bitcoin address from the attestor network

**What happens after you send BTC:**

- You send BTC to the address (external to this library)
- The attestor network monitors Bitcoin transactions
- After 6+ confirmations, attestors automatically mint CBTC directly to your Canton party
- No monitoring is implemented in this library - the attestors handle everything

## Step-by-Step API Flow

### 1. Authentication (Keycloak OAuth2 Password Grant)

**Endpoint:**

```
POST {keycloak_host}/auth/realms/{realm}/protocol/openid-connect/token
```

**Headers:**

```
Content-Type: application/x-www-form-urlencoded
```

**Request Body (form-encoded):**

```
grant_type=password
client_id={client_id}
username={username}
password={password}
```

**Response:**

```json
{
  "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 300,
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer"
}
```

**Notes:**

- The `access_token` is a JWT that must be included as a Bearer token in all subsequent Canton API calls
- Token typically expires in 5 minutes (300 seconds)
- The JWT contains a `sub` claim with the user's UUID, which is extracted and used as `user_id` in Canton submissions
- Use the `refresh_token` to get a new access token without re-authenticating

---

### 2. Get Account Contract Rules from Attestor

**Endpoint:**

```
POST {attestor_url}/app/get-account-contract-rules
```

**Headers:**

```
Content-Type: application/json
```

**Request Body:**

```json
{
  "chain": "canton-devnet"
}
```

**Response:**

```json
{
  "da_rules": {
    "template_id": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.DepositAccount:CBTCDepositAccountRules",
    "contract_id": "0085259d857d8808aaec759c9b734651e7e7f0e567b136fb79177edef3e23f8f7bca111220d3368772598c96b74570c3ba7b092cfb7549ab58c6e248b339eda9998091e597",
    "created_event_blob": "CgMyLjES/QQKRQCFJZ2FfYgIqux1nJtzRlHn5/DlZ7E2+3kXft7z4j+Pe8oREiDTNodyWYyWt0Vww7p7CSz7dUmrWMbiSLM57amZgJHllxIEY2J0YxpxCkA0M2E4NDUyYTU2Mzg4ZDIyYzYwNThhYmUwM2U5MGRhZGJhZTlhMjBhNjgyNjM0NTY4YjA3YTkzNTMxZGRhMWEzEgRDQlRDEg5EZXBvc2l0QWNjb3VudBoXQ0JUQ0RlcG9zaXRBY2NvdW50UnVsZXMisQJqrgIKVgpUOlJjYnRjLW5ldHdvcms6OjEyMjA1YWYzYjk0OWEwNDc3NmZjNDhjZGNjMDVhMDYwZjZiZGEyZTQ3MDYzMjkzNWYzNzVkMTA0OWE4NTQ2YTNiMjYyCmwKajpoYXV0aDBfMDA3YzY2NDM1MzhmMmVhZGQzZTU3M2RkMDViOTo6MTIyMDViY2MxMDZlZmEwZWFhN2YxOGRjNDkxZTVjNmY1ZmI5YjBjYzY4ZGMxMTBhZTY2ZjRlZDY0Njc0NzVkN2M3OGUKZgpkamIKVgpUOlJjYnRjLW5ldHdvcms6OjEyMjA1YWYzYjk0OWEwNDc3NmZjNDhjZGNjMDVhMDYwZjZiZGEyZTQ3MDYzMjkzNWYzNzVkMTA0OWE4NTQ2YTNiMjYyCggKBkIEQ0JUQypSY2J0Yy1uZXR3b3JrOjoxMjIwNWFmM2I5NDlhMDQ3NzZmYzQ4Y2RjYzA1YTA2MGY2YmRhMmU0NzA2MzI5MzVmMzc1ZDEwNDlhODU0NmEzYjI2MjlxhYrw4T0GAEIqCiYKJAgBEiD/RkOrqwY2xAum/h6CtY8yKisHs/XfQscIYoAxS53Y7BAe"
  },
  "wa_rules": {
    "template_id": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
    "contract_id": "00c007c8b886320cf18412333c614bc5a0b098015992aadc8387a6ed60139c7305ca1112208ae086cfa20aedaa1461a33f8569f148e100eb15280f7859f4f1985bc4033489",
    "created_event_blob": "CgMyLjES/wQKRQDAB8i4hjIM8YQSMzxhS8WgsJgBWZKq3IOHpu1gE5xzBcoREiCK4IbPogrtqhRhoz+FafFI4QDrFSgPeFn08ZhbxAM0iRIEY2J0YxpzCkA0M2E4NDUyYTU2Mzg4ZDIyYzYwNThhYmUwM2U5MGRhZGJhZTlhMjBhNjgyNjM0NTY4YjA3YTkzNTMxZGRhMWEzEgRDQlRDEg9XaXRoZHJhd0FjY291bnQaGENCVENXaXRoZHJhd0FjY291bnRSdWxlcyKxAmquAgpWClQ6UmNidGMtbmV0d29yazo6MTIyMDVhZjNiOTQ5YTA0Nzc2ZmM0OGNkY2MwNWEwNjBmNmJkYTJlNDcwNjMyOTM1ZjM3NWQxMDQ5YTg1NDZhM2IyNjIKbApqOmhhdXRoMF8wMDdjNjY0MzUzOGYyZWFkZDNlNTczZGQwNWI5OjoxMjIwNWJjYzEwNmVmYTBlYWE3ZjE4ZGM0OTFlNWM2ZjVmYjliMGNjNjhkYzExMGFlNjZmNGVkNjQ2NzQ3NWQ3Yzc4ZQpmCmRqYgpWClQ6UmNidGMtbmV0d29yazo6MTIyMDVhZjNiOTQ5YTA0Nzc2ZmM0OGNkY2MwNWEwNjBmNmJkYTJlNDcwNjMyOTM1ZjM3NWQxMDQ5YTg1NDZhM2IyNjIKCAoGQgRDQlRDKlJjYnRjLW5ldHdvcms6OjEyMjA1YWYzYjk0OWEwNDc3NmZjNDhjZGNjMDVhMDYwZjZiZGEyZTQ3MDYzMjkzNWYzNzVkMTA0OWE4NTQ2YTNiMjYyOXhwmvDhPQYAQioKJgokCAESINqYs9g9Cgfl5Qt/FsMAv3jECAI3uLohwPKxeBLEmG5aEB4="
  }
}
```

**Notes:**

- `da_rules` = DepositAccountRules contract that governs deposit account creation
- `wa_rules` = WithdrawAccountRules contract (used in redeem/burn flow)
- The `created_event_blob` is a base64-encoded serialized contract used for Canton's disclosed contracts mechanism
- These rules are singleton contracts maintained by the attestor network
- Template IDs and contract IDs are unique to each environment (devnet/testnet/mainnet)

---

### 3. Get Canton Ledger End Offset

**Endpoint:**

```
GET {ledger_host}/v2/state/ledger-end
```

**Headers:**

```
Authorization: Bearer {access_token}
```

**Response:**

```json
{
  "offset": 12345678
}
```

**Notes:**

- The ledger offset is required for querying active contracts
- This ensures you're querying at a specific point in the ledger's history
- Must be called before each `POST /v2/state/active-contracts` query

---

### 4. Create Deposit Account on Canton

**Endpoint:**

```
POST {ledger_host}/v2/commands/submit-and-wait-for-transaction-tree
```

**Headers:**

```
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:**

```json
{
  "actAs": ["party::1220abc..."],
  "commandId": "cmd-550e8400-e29b-41d4-a716-446655440000",
  "disclosedContracts": [
    {
      "contractId": "00abc123...",
      "createdEventBlob": "base64-encoded-da-rules-contract...",
      "templateId": "Splice.DsoRules:DsoRules",
      "synchronizerId": ""
    }
  ],
  "commands": [
    {
      "ExerciseCommand": {
        "exerciseCommand": {
          "templateId": "Splice.DsoRules:DsoRules",
          "contractId": "00abc123...",
          "choice": "CreateDepositAccount",
          "choiceArgument": {
            "owner": "party::1220abc..."
          }
        }
      }
    }
  ],
  "readAs": ["party::1220abc..."],
  "userId": "user-uuid-from-jwt-sub-claim"
}
```

**Response:**

```json
{
  "transactionTree": {
    "eventsById": {
      "evt-123": {
        "CreatedTreeEvent": {
          "value": {
            "contractId": "00depositabc123...",
            "templateId": "...CBTC.DepositAccount:CBTCDepositAccount",
            "createArgument": {
              "id": "550e8400-e29b-41d4-a716-446655440001",
              "owner": "party::1220abc...",
              "operator": "party::1220operator...",
              "registrar": "party::1220registrar...",
              "lastProcessedBitcoinBlock": 0
            },
            "createdEventBlob": "base64-encoded-deposit-account..."
          }
        }
      }
    },
    "updateId": "update-id-123",
    "commandId": "cmd-550e8400-e29b-41d4-a716-446655440000",
    "offset": "12345679"
  }
}
```

**Notes:**

- `actAs`: The party creating the account (must match authenticated user's party)
- `commandId`: Unique identifier for idempotency (use UUID v4)
- `disclosedContracts`: Must include the DepositAccountRules contract from step 2
- `choice`: "CreateDepositAccount" is the Canton choice being exercised
- `userId`: Extracted from JWT's `sub` claim
- Response contains the created DepositAccount contract with its `id` field (UUID)
- The `id` field in the contract's `createArgument` is what you'll use to get the Bitcoin address

---

### 5. Get Bitcoin Address for Deposit Account

**Endpoint:**

```
POST {attestor_url}/app/get-bitcoin-address
```

**Headers:**

```
Content-Type: application/json
```

**Request Body:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "chain": "canton-devnet"
}
```

**Response:**

```
bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3
```

**Notes:**

- `id`: The UUID from the DepositAccount contract's `id` field (NOT the contract_id)
- Response is plain text containing just the Bitcoin address
- This address is unique to your deposit account
- Any BTC sent to this address will be tracked and converted to CBTC after 6+ confirmations

---

### 6. Send Bitcoin (External)

**Action:** Send BTC to the address from step 5 using your Bitcoin wallet or service.

**Notes:**

- This step is external to the API
- The attestor network monitors Bitcoin for transactions to this address
- Requires 6+ Bitcoin confirmations before CBTC is minted
- Multiple deposits to the same address are supported
- After 6+ confirmations, the attestors automatically mint CBTC tokens directly to your party
- **No DepositRequest contracts are created** - CBTC is minted directly

---

### 7. Verify Minted CBTC (Optional)

Once the attestor has minted CBTC after 6+ confirmations, you can verify your balance by listing your CBTC holdings.

**Quick Check**: You can use the `check_balance` example to monitor for minted CBTC:
```bash
cargo run -p examples --bin check_balance
```

**Endpoint:**

```
POST {ledger_host}/v2/state/active-contracts
```

**Headers:**

```
Authorization: Bearer {access_token}
Content-Type: application/json
```

**Request Body:**

```json
{
  "filter": {
    "filtersByParty": {
      "party::1220abc...": {
        "cumulative": [
          {
            "identifierFilter": {
              "TemplateIdentifierFilter": {
                "templateFilter": {
                  "value": {
                    "templateId": "...Splice.Wallet:Holding",
                    "includeCreatedEventBlob": true
                  }
                }
              }
            }
          }
        ]
      }
    }
  },
  "verbose": false,
  "activeAtOffset": 12345680
}
```

**Notes:**

- This queries for Holding contracts which represent CBTC UTXOs
- Filter holdings where `instrument.id == "CBTC"` to see only CBTC holdings
- Sum the `amount` fields to get your total CBTC balance
- This is the same query used by the `redeem::list_holdings()` function

---

## Summary of Endpoints

### Keycloak OAuth

- `POST {keycloak_host}/auth/realms/{realm}/protocol/openid-connect/token` - Get JWT token

### Attestor Network

- `POST {attestor_url}/app/get-account-contract-rules` - Get DepositAccountRules and WithdrawAccountRules
- `POST {attestor_url}/app/get-bitcoin-address` - Get Bitcoin address for deposit account

### Canton Ledger

- `GET {ledger_host}/v2/state/ledger-end` - Get current ledger offset
- `POST {ledger_host}/v2/commands/submit-and-wait-for-transaction-tree` - Create deposit account
- `POST {ledger_host}/v2/state/active-contracts` - Query active contracts (deposit accounts, holdings, etc.)

---

## Key Data Structures

### DepositAccount Contract

```
Template: CBTC.DepositAccount:CBTCDepositAccount
Fields:
  - owner: Party (your party ID)
  - operator: Party (attestor's party)
  - registrar: Party (attestor's party)
  - lastProcessedBitcoinBlock: String (last scanned BTC block height)
```

**Note**: The contract_id of the DepositAccount is used to get the Bitcoin address from the attestor.

### Holding Contract (CBTC Tokens)

```
Template: Splice.Wallet:Holding
Fields:
  - owner: Party (token owner)
  - amount: String (CBTC amount in decimal format)
  - instrument: Object
    - id: String (e.g., "CBTC")
  - lock: Optional (null if unlocked, present if locked in a transaction)
```

**Note**: After attestors mint CBTC, it appears as Holding contracts. These are UTXO-style tokens that can be transferred or burned.

---

## Notes for Implementers

1. **JWT Expiration**: Access tokens expire quickly (typically 5 minutes). Implement token refresh logic using the `refresh_token` from the initial authentication.

2. **No Monitoring Needed**: This library does NOT monitor Bitcoin transactions or deposits. The attestor network handles all monitoring and automatically mints CBTC after 6+ confirmations. You only need to create the deposit account and provide the user with the Bitcoin address.

3. **Bitcoin Confirmations**: The attestor requires 6+ Bitcoin confirmations before minting CBTC. This typically takes ~60 minutes.

4. **Template ID Matching**: Template IDs have different prefixes in different environments. When querying contracts, match by suffix (e.g., ends with `:CBTC.DepositAccount:CBTCDepositAccount`).

5. **Disclosed Contracts**: The `disclosedContracts` array is required for Canton's multi-party authorization. Always include the relevant rules contracts from the attestor.

6. **Command IDs**: Use UUIDs for `commandId` to ensure idempotency. If a command is submitted multiple times with the same ID, Canton will only execute it once.

7. **Error Handling**: All endpoints can return errors. HTTP 4xx/5xx status codes indicate failures. Parse error messages from response bodies.

8. **Network-Specific Values**: Template IDs, party IDs, and contract IDs vary by network (devnet/testnet/mainnet). Always fetch these from the attestor's `get-account-contract-rules` endpoint.
