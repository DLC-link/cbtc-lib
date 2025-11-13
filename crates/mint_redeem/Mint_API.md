# CBTC Mint API Flow

This document details the underlying HTTP APIs and Canton contracts used to mint CBTC from Bitcoin. This is intended for developers implementing the mint flow in other languages or debugging issues.

## Overview

The mint flow converts Bitcoin (BTC) into Canton Bitcoin (CBTC) by:
1. Authenticating with Keycloak to get a JWT access token
2. Creating a DepositAccount contract on Canton
3. Getting a Bitcoin address from the attestor network
4. Sending BTC to that address (external to this library)
5. Monitoring for DepositRequest contracts that represent confirmed deposits

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
    "contract_id": "00abc123...",
    "template_id": "Splice.DsoRules:DsoRules",
    "created_event_blob": "base64-encoded-contract-data..."
  },
  "wa_rules": {
    "contract_id": "00def456...",
    "template_id": "Splice.DsoRules:DsoRules",
    "created_event_blob": "base64-encoded-contract-data..."
  }
}
```

**Notes:**
- `da_rules` = DepositAccountRules contract that governs deposit account creation
- `wa_rules` = WithdrawAccountRules contract (used in burn flow)
- The `created_event_blob` is a base64-encoded serialized contract used for Canton's disclosed contracts mechanism
- These rules are singleton contracts maintained by the attestor network

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

---

### 7. Monitor for Deposit Requests (Polling)

#### 7a. Get Ledger End Offset (repeat)

Same as step 3.

#### 7b. Query Active DepositRequest Contracts

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
                    "templateId": "...CBTC.DepositRequest:CBTCDepositRequest",
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

**Response:**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00deposit-request-123...",
          "templateId": "...CBTC.DepositRequest:CBTCDepositRequest",
          "createArgument": {
            "id": "550e8400-e29b-41d4-a716-446655440002",
            "depositAccountId": "550e8400-e29b-41d4-a716-446655440001",
            "owner": "party::1220abc...",
            "amount": "0.001",
            "btcTxId": "abc123def456...",
            "processed": false
          },
          "createdEventBlob": "base64-encoded..."
        },
        "reassignmentCounter": 0,
        "synchronizerId": ""
      }
    }
  }
]
```

**Notes:**
- Poll this endpoint periodically (e.g., every 30 seconds) to detect new deposits
- `templateId`: Filter for DepositRequest contracts using the template ID suffix `:CBTC.DepositRequest:CBTCDepositRequest`
- When a DepositRequest appears, it means:
  - BTC was detected at your address
  - It has 6+ confirmations
  - CBTC has been automatically minted to your party
- `amount`: The amount of BTC/CBTC (in BTC units)
- `btcTxId`: The Bitcoin transaction ID
- `depositAccountId`: Links back to your DepositAccount's `id`

---

### 8. Check Deposit Account Status (Optional)

This combines steps 3, 7b, and 5 to get a unified view of the deposit account including pending balance.

#### 8a. Get Ledger End Offset

Same as step 3.

#### 8b. Query Active DepositAccount Contracts

**Endpoint:**
```
POST {ledger_host}/v2/state/active-contracts
```

**Request:** Similar to 7b but filter by DepositAccount template ID:
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
                    "templateId": "...CBTC.DepositAccount:CBTCDepositAccount",
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
  "activeAtOffset": 12345681
}
```

**Response:** Returns list of DepositAccount contracts (find the one matching your `contract_id`)

#### 8c. Get Bitcoin Address

Same as step 5.

**Combined Status Information:**
- `contract_id`: From Canton query (8b)
- `owner`, `operator`, `registrar`: From contract's createArgument
- `bitcoin_address`: From attestor (8c)
- `last_processed_bitcoin_block`: From contract's createArgument
- `pending_balance`: NOT directly available via this flow (would require additional attestor endpoint)

---

## Summary of Endpoints

### Keycloak OAuth
- `POST {keycloak_host}/auth/realms/{realm}/protocol/openid-connect/token` - Get JWT token

### Attestor Network
- `POST {attestor_url}/app/get-account-contract-rules` - Get DepositAccountRules contract
- `POST {attestor_url}/app/get-bitcoin-address` - Get Bitcoin address for account

### Canton Ledger
- `GET {ledger_host}/v2/state/ledger-end` - Get current ledger offset
- `POST {ledger_host}/v2/commands/submit-and-wait-for-transaction-tree` - Execute contract choices
- `POST {ledger_host}/v2/state/active-contracts` - Query active contracts

---

## Key Data Structures

### DepositAccount Contract
```
Template: CBTC.DepositAccount:CBTCDepositAccount
Fields:
  - id: UUID (used to get Bitcoin address)
  - owner: Party (your party ID)
  - operator: Party (attestor's party)
  - registrar: Party (attestor's party)
  - lastProcessedBitcoinBlock: Int (last scanned BTC block)
```

### DepositRequest Contract
```
Template: CBTC.DepositRequest:CBTCDepositRequest
Fields:
  - id: UUID
  - depositAccountId: UUID (links to DepositAccount)
  - owner: Party (your party)
  - amount: String (BTC amount in decimal format)
  - btcTxId: String (Bitcoin transaction ID)
  - processed: Bool
```

---

## Notes for Implementers

1. **JWT Expiration**: Access tokens expire quickly (typically 5 minutes). Implement token refresh logic using the `refresh_token` from the initial authentication.

2. **Polling Frequency**: Poll for DepositRequests every 30-60 seconds. Don't poll too frequently to avoid rate limiting.

3. **Bitcoin Confirmations**: The attestor requires 6+ Bitcoin confirmations before minting CBTC. This typically takes ~60 minutes.

4. **Template ID Matching**: Template IDs may have different prefixes in different environments. Always match by suffix (e.g., ends with `:CBTC.DepositAccount:CBTCDepositAccount`).

5. **Disclosed Contracts**: The `disclosedContracts` array is required for Canton's multi-party authorization. Always include the relevant rules contracts.

6. **Command IDs**: Use UUIDs for `commandId` to ensure idempotency. If a command is submitted multiple times with the same ID, Canton will only execute it once.

7. **Error Handling**: All endpoints can return errors. HTTP 4xx/5xx status codes indicate failures. Parse error messages from response bodies.

8. **Network-Specific Values**: Template IDs, party IDs, and contract IDs vary by network (devnet/testnet/mainnet). Don't hardcode these values.
