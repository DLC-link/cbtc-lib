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

**Example Response:**

```json
{
  "da_rules": {
    "template_id": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.DepositAccount:CBTCDepositAccountRules",
    "contract_id": "0085259d857d8808aaec759c9b734651e7e7f0e567b136fb79177edef3e23f8f7b...",
    "created_event_blob": "CgMyLjES/QQKRQCFJZ2FfYgIqux1nJtzRlHn5/DlZ7E2+3kXft7z4j+Pe8o..."
  },
  "wa_rules": {
    "template_id": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
    "contract_id": "00c007c8b886320cf18412333c614bc5a0b098015992aadc8387a6ed60139c730...",
    "created_event_blob": "CgMyLjES/wQKRQDAB8i4hjIM8YQSMzxhS8WgsJgBWZKq3IOHpu1gE5xzBco..."
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

**Example Request:**

```json
{
  "commands": [
    {
      "ExerciseCommand": {
        "templateId": "#cbtc:CBTC.DepositAccount:CBTCDepositAccountRules",
        "contractId": "0085259d857d8808aaec759c9b734651e7e7f0e567b136fb79177edef3e23f8f7b...",
        "choice": "CBTCDepositAccountRules_CreateDepositAccount",
        "choiceArgument": {
          "owner": "your-party::1220abcdef..."
        }
      }
    }
  ],
  "actAs": [
    "your-party::1220abcdef..."
  ],
  "commandId": "cmd-550e8400-e29b-41d4-a716-446655440000",
  "disclosedContracts": [
    {
      "templateId": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.DepositAccount:CBTCDepositAccountRules",
      "contractId": "0085259d857d8808aaec759c9b734651e7e7f0e567b136fb79177edef3e23f8f7b...",
      "createdEventBlob": "CgMyLjES/QQKRQCFJZ2FfYgIqux1nJtzRlHn5/DlZ7E2+3kXft7z4j+Pe8o...",
      "synchronizerId": ""
    }
  ]
}
```

**Example Response:**

```json
{
  "transactionTree": {
    "updateId": "1220013c383fa00c0fb34ca0b690ff63daa3be4c23ba8a8c8d3997733b92479f2d7d",
    "commandId": "cmd-550e8400-e29b-41d4-a716-446655440000",
    "workflowId": "",
    "effectiveAt": "2025-11-05T09:23:32.018640Z",
    "offset": 3861240,
    "eventsById": {
      "0": {
        "ExercisedTreeEvent": {
          "value": {
            "contractId": "00658ca0d18a99a414de9f3d8bc40b01c21fceabac410cdafc637d3af79966187...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.DepositAccount:CBTCDepositAccountRules",
            "choice": "CBTCDepositAccountRules_CreateDepositAccount",
            "choiceArgument": {
              "owner": "your-party::1220abcdef..."
            },
            "actingParties": ["your-party::1220abcdef..."],
            "consuming": false,
            "exerciseResult": {
              "depositAccountCid": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85..."
            },
            "packageName": "cbtc"
          }
        }
      },
      "1": {
        "CreatedTreeEvent": {
          "value": {
            "contractId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.DepositAccount:CBTCDepositAccount",
            "createArgument": {
              "registrar": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
              "operator": "operator-party::1220...",
              "instrument": {
                "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
                "id": "CBTC"
              },
              "owner": "your-party::1220abcdef...",
              "id": null,
              "lastProcessedBitcoinBlock": "0"
            },
            "createdEventBlob": "",
            "witnessParties": ["your-party::1220abcdef..."],
            "signatories": [
              "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
              "your-party::1220abcdef..."
            ],
            "observers": [],
            "createdAt": "2025-11-05T09:23:32.018640Z",
            "packageName": "cbtc"
          }
        }
      }
    },
    "synchronizerId": "global-domain::1220...",
    "recordTime": "2025-11-05T09:23:32.689031Z"
  }
}
```

**Notes:**

**Request:**
- `templateId` in `ExerciseCommand` uses shorthand format `#cbtc:CBTC.DepositAccount:CBTCDepositAccountRules`
- `choice` is `CBTCDepositAccountRules_CreateDepositAccount` (includes the template name prefix)
- `actAs`: The party creating the account (must match authenticated user's party)
- `commandId`: Unique identifier for idempotency (use UUID v4)
- `disclosedContracts`: Must include the full DepositAccountRules contract from step 2
- `disclosedContracts[].templateId` uses full hash format (different from ExerciseCommand templateId)
- `disclosedContracts[].synchronizerId` should be an empty string

**Response:**
- Contains two events in `eventsById`: `ExercisedTreeEvent` (index "0") and `CreatedTreeEvent` (index "1")
- The `ExercisedTreeEvent` shows which choice was exercised on the rules contract
- The `CreatedTreeEvent` contains the actual DepositAccount contract that was created
- `createArgument.id` can be `null` or a UUID string
- `createArgument.instrument` contains the token information with `id: "CBTC"`
- `createArgument.lastProcessedBitcoinBlock` is a string `"0"`, not an integer
- **To get Bitcoin address**: Use `createArgument.id` if it's not null, otherwise use `contractId` from the CreatedTreeEvent

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

**Example Request:**

```json
{
  "id": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
  "chain": "canton-mainnet"
}
```

**Response:**

```
bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3
```

**Notes:**

- `id`: Use the UUID from `createArgument.id` if it's not null, otherwise use the `contractId` from the CreatedTreeEvent
- `chain`: The Canton network identifier (e.g., "canton-devnet", "canton-testnet", "canton-mainnet")
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

**Example Request:**

```json
{
  "verbose": true,
  "activeAtOffset": 3896033,
  "filter": {
    "filtersByParty": {
      "your-party::1220abcdef...": {
        "cumulative": [
          {
            "identifierFilter": {
              "InterfaceFilter": {
                "value": {
                  "interfaceId": "#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding",
                  "includeInterfaceView": true,
                  "includeCreatedEventBlob": true
                }
              }
            }
          }
        ]
      }
    }
  }
}
```

**Notes:**

- **IMPORTANT**: There is no `Splice.Wallet:Holding` template - holdings are accessed via the `#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding` interface
- Uses `InterfaceFilter` (not `TemplateIdentifierFilter`) to query via Canton's interface system
- `interfaceId` uses shorthand format: `#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding`
- `includeInterfaceView: true` includes the interface view in the response
- `includeCreatedEventBlob: true` includes the blob for disclosed contracts
- `verbose: true` returns full contract details
- `activeAtOffset` requires getting the ledger end offset first (step 3)
- This queries for contracts that implement the HoldingV1 interface, which represent CBTC UTXOs
- The actual template ID in responses will vary by implementation, but all holdings expose the standard `HoldingV1` interface
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
  - id: String | null (UUID if present, null otherwise)
  - owner: Party (your party ID)
  - operator: Party (attestor's party)
  - registrar: Party (attestor's party)
  - instrument: Object
    - id: String (e.g., "CBTC")
    - admin: Party (CBTC network party)
  - lastProcessedBitcoinBlock: String (last scanned BTC block height, e.g., "0")
```

**Note**: To get the Bitcoin address from the attestor, use the `id` field if it's not null, otherwise use the `contractId`.

### Holding Contract (CBTC Tokens)

```
Interface: #splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding
Note: There is no specific "Holding" template - various templates implement this interface
Fields (via HoldingV1 interface):
  - owner: Party (token owner)
  - amount: String (CBTC amount in decimal format)
  - instrument: Object
    - id: String (e.g., "CBTC")
    - admin: Party (CBTC network party)
  - lock: Optional (null if unlocked, present if locked in a transaction)
```

**Important**: Holdings are queried using `InterfaceFilter` with the `HoldingV1` interface, not by a specific template. The actual template ID will vary depending on the implementation, but all holdings expose the standard interface fields listed above.

**Note**: After attestors mint CBTC, it appears as contracts implementing the HoldingV1 interface. These are UTXO-style tokens that can be transferred or burned.

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
