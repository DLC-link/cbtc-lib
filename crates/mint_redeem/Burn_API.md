# CBTC Burn/Redeem API Flow

This document details the underlying HTTP APIs and Canton contracts used to burn CBTC and redeem Bitcoin. This is intended for developers implementing the burn/redeem flow in other languages or debugging issues.

## Overview

The burn/redeem flow converts Canton Bitcoin (CBTC) back into Bitcoin (BTC) by:
1. Authenticating with Keycloak to get a JWT access token
2. Creating a WithdrawAccount contract on Canton with a destination Bitcoin address
3. Selecting CBTC holdings (UTXOs) to burn
4. Getting token standard contracts from the attestor
5. Burning CBTC by exercising the "Withdraw" choice on the WithdrawAccount
6. Monitoring for the attestor to process the withdrawal and send BTC

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
- Extract the `sub` claim from the JWT to get the user's UUID (needed for Canton submissions)
- To extract: base64-decode the middle part of the JWT, parse as JSON, read `sub` field

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
- `wa_rules` = WithdrawAccountRules contract that governs withdraw account creation
- The `created_event_blob` is required for Canton's disclosed contracts mechanism

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
- Required before querying active contracts
- Must be called fresh for each query to ensure consistency

---

### 4. Create Withdraw Account on Canton

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
      "contractId": "00def456...",
      "createdEventBlob": "base64-encoded-wa-rules-contract...",
      "templateId": "Splice.DsoRules:DsoRules",
      "synchronizerId": ""
    }
  ],
  "commands": [
    {
      "ExerciseCommand": {
        "exerciseCommand": {
          "templateId": "Splice.DsoRules:DsoRules",
          "contractId": "00def456...",
          "choice": "CreateWithdrawAccount",
          "choiceArgument": {
            "owner": "party::1220abc...",
            "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3"
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
            "contractId": "00withdrawabc123...",
            "templateId": "...CBTC.WithdrawAccount:CBTCWithdrawAccount",
            "createArgument": {
              "id": "550e8400-e29b-41d4-a716-446655440001",
              "owner": "party::1220abc...",
              "operator": "party::1220operator...",
              "registrar": "party::1220registrar...",
              "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3"
            },
            "createdEventBlob": "base64-encoded-withdraw-account..."
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
- `actAs`: The party creating the account
- `disclosedContracts`: Must include the WithdrawAccountRules contract from step 2
- `choice`: "CreateWithdrawAccount" is the Canton choice being exercised
- `destinationBtcAddress`: The Bitcoin address where BTC will be sent (locked at creation time)
- `userId`: Extracted from JWT's `sub` claim
- Response contains the created WithdrawAccount contract

---

### 5. List CBTC Holdings (Query UTXOs)

#### 5a. Get Ledger End Offset

Same as step 3.

#### 5b. Query Active Holding Contracts

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

**Response:**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00holding123...",
          "templateId": "...Splice.Wallet:Holding",
          "createArgument": {
            "instrument": {
              "id": {
                "unpack": "CBTC"
              },
              "version": "0"
            },
            "amount": {
              "quantity": "0.001"
            },
            "owner": "party::1220abc...",
            "lock": null
          },
          "createdEventBlob": "base64-encoded..."
        },
        "reassignmentCounter": 0,
        "synchronizerId": ""
      }
    }
  },
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00holding456...",
          "templateId": "...Splice.Wallet:Holding",
          "createArgument": {
            "instrument": {
              "id": {
                "unpack": "CBTC"
              },
              "version": "0"
            },
            "amount": {
              "quantity": "0.002"
            },
            "owner": "party::1220abc...",
            "lock": null
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
- Holdings are UTXO-based like Bitcoin - each holding is a separate contract
- Filter for `instrument.id.unpack == "CBTC"` and `owner == your_party`
- Exclude holdings where `lock != null` (locked holdings are unavailable)
- Select enough holdings to cover your burn amount (similar to Bitcoin UTXO selection)
- Collect the `contractId` values for the holdings you want to burn

---

### 6. Get Token Standard Contracts from Attestor

**Endpoint:**
```
POST {attestor_url}/app/get-token-standard-contracts
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
  "burn_mint_factory": {
    "contract_id": "00factory123...",
    "template_id": "...Splice.TokenStandard:BurnMintFactory",
    "created_event_blob": "base64-encoded..."
  },
  "instrument_configuration": {
    "contract_id": "00instrument123...",
    "template_id": "...Splice.TokenStandard:InstrumentConfiguration",
    "created_event_blob": "base64-encoded..."
  },
  "issuer_credential": {
    "contract_id": "00issuer123...",
    "template_id": "...Splice.TokenStandard:IssuerCredential",
    "created_event_blob": "base64-encoded..."
  },
  "app_reward_configuration": {
    "contract_id": "00reward123...",
    "template_id": "...Splice.TokenStandard:AppRewardConfiguration",
    "created_event_blob": "base64-encoded..."
  },
  "featured_app_right": {
    "contract_id": "00featured123...",
    "template_id": "...Splice.TokenStandard:FeaturedAppRight",
    "created_event_blob": "base64-encoded..."
  }
}
```

**Notes:**
- These contracts are required by Canton's token standard (CIP-0056) for burn operations
- `burn_mint_factory` and `instrument_configuration` are always required
- Other fields (`issuer_credential`, `app_reward_configuration`, `featured_app_right`) may be null/optional
- All non-null contracts must be included in the disclosed contracts for the burn transaction

---

### 7. Burn CBTC and Request Withdrawal

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
  "commandId": "cmd-550e8400-e29b-41d4-a716-446655440001",
  "disclosedContracts": [
    {
      "contractId": "00factory123...",
      "createdEventBlob": "base64-encoded-burn-mint-factory...",
      "templateId": "...Splice.TokenStandard:BurnMintFactory",
      "synchronizerId": ""
    },
    {
      "contractId": "00instrument123...",
      "createdEventBlob": "base64-encoded-instrument-configuration...",
      "templateId": "...Splice.TokenStandard:InstrumentConfiguration",
      "synchronizerId": ""
    },
    {
      "contractId": "00issuer123...",
      "createdEventBlob": "base64-encoded-issuer-credential...",
      "templateId": "...Splice.TokenStandard:IssuerCredential",
      "synchronizerId": ""
    }
  ],
  "commands": [
    {
      "ExerciseCommand": {
        "exerciseCommand": {
          "templateId": "...CBTC.WithdrawAccount:CBTCWithdrawAccount",
          "contractId": "00withdrawabc123...",
          "choice": "Withdraw",
          "choiceArgument": {
            "tokens": [
              "00holding123...",
              "00holding456..."
            ],
            "amount": "0.001",
            "burnMintFactoryCid": "00factory123...",
            "extraArgs": {
              "context": {
                "values": {
                  "utility.digitalasset.com/instrument-configuration": {
                    "tag": "AV_ContractId",
                    "value": "00instrument123..."
                  },
                  "utility.digitalasset.com/issuer-credentials": {
                    "tag": "AV_List",
                    "value": [
                      {
                        "tag": "AV_ContractId",
                        "value": "00issuer123..."
                      }
                    ]
                  },
                  "utility.digitalasset.com/app-reward-configuration": {
                    "tag": "AV_ContractId",
                    "value": "00reward123..."
                  },
                  "utility.digitalasset.com/featured-app-right": {
                    "tag": "AV_ContractId",
                    "value": "00featured123..."
                  }
                }
              },
              "meta": {
                "values": {
                  "splice.lfdecentralizedtrust.org/reason": "CBTC withdrawal"
                }
              }
            }
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
      "evt-456": {
        "CreatedTreeEvent": {
          "value": {
            "contractId": "00withdraw-request-123...",
            "templateId": "...CBTC.WithdrawRequest:CBTCWithdrawRequest",
            "createArgument": {
              "id": "550e8400-e29b-41d4-a716-446655440002",
              "withdrawAccountId": "550e8400-e29b-41d4-a716-446655440001",
              "owner": "party::1220abc...",
              "amount": "0.001",
              "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
              "btcTxId": null,
              "processed": false
            },
            "createdEventBlob": "base64-encoded-withdraw-request..."
          }
        }
      },
      "evt-789": {
        "ExercisedTreeEvent": {
          "contractId": "00holding123...",
          "templateId": "...Splice.Wallet:Holding",
          "choice": "Archive"
        }
      },
      "evt-790": {
        "ExercisedTreeEvent": {
          "contractId": "00holding456...",
          "templateId": "...Splice.Wallet:Holding",
          "choice": "Archive"
        }
      }
    },
    "updateId": "update-id-456",
    "commandId": "cmd-550e8400-e29b-41d4-a716-446655440001",
    "offset": "12345681"
  }
}
```

**Notes:**
- `choice`: "Withdraw" burns the CBTC and creates a WithdrawRequest
- `tokens`: Array of holding contract IDs to burn (must have sufficient total amount)
- `amount`: The amount to burn (must be <= sum of holding amounts)
- `burnMintFactoryCid`: The burn_mint_factory contract ID from step 6
- `extraArgs.context`: Token standard context with all required contract references
  - `instrument-configuration`: Always required
  - `issuer-credentials`: List of issuer credentials (if present)
  - `app-reward-configuration`: App reward config (if present)
  - `featured-app-right`: Featured app right (if present)
- `extraArgs.meta.values`: Metadata about the operation
- **IMPORTANT**: The `amount` field must be a quoted decimal string (not scientific notation) or Canton will reject it
- `disclosedContracts`: Must include all token standard contracts (burn_mint_factory, instrument_configuration, and any optional ones)
- `userId`: Extracted from JWT's `sub` claim
- Response shows:
  - Created WithdrawRequest contract (with `btcTxId: null` initially)
  - Archived (burned) holding contracts
  - Any change holdings if amount < sum of selected holdings

---

### 8. Monitor Withdrawal Status (Polling)

#### 8a. Get Ledger End Offset

Same as step 3.

#### 8b. Query Active WithdrawRequest Contracts

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
                    "templateId": "...CBTC.WithdrawRequest:CBTCWithdrawRequest",
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
  "activeAtOffset": 12345682
}
```

**Response (initially):**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00withdraw-request-123...",
          "templateId": "...CBTC.WithdrawRequest:CBTCWithdrawRequest",
          "createArgument": {
            "id": "550e8400-e29b-41d4-a716-446655440002",
            "withdrawAccountId": "550e8400-e29b-41d4-a716-446655440001",
            "owner": "party::1220abc...",
            "amount": "0.001",
            "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
            "btcTxId": null,
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

**Response (after attestor processing):**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00withdraw-request-123...",
          "templateId": "...CBTC.WithdrawRequest:CBTCWithdrawRequest",
          "createArgument": {
            "id": "550e8400-e29b-41d4-a716-446655440002",
            "withdrawAccountId": "550e8400-e29b-41d4-a716-446655440001",
            "owner": "party::1220abc...",
            "amount": "0.001",
            "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
            "btcTxId": "def456abc789...",
            "processed": true
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
- Poll this endpoint periodically (e.g., every 30 seconds) to check withdrawal status
- Initially `btcTxId` will be `null` and `processed` will be `false`
- Once the attestor processes the withdrawal:
  - `btcTxId` is set to the Bitcoin transaction ID
  - `processed` is set to `true`
  - BTC has been sent to the `destinationBtcAddress`
- The attestor typically processes withdrawals within a few minutes

---

## Summary of Endpoints

### Keycloak OAuth
- `POST {keycloak_host}/auth/realms/{realm}/protocol/openid-connect/token` - Get JWT token

### Attestor Network
- `POST {attestor_url}/app/get-account-contract-rules` - Get WithdrawAccountRules contract
- `POST {attestor_url}/app/get-token-standard-contracts` - Get token standard contracts for burning

### Canton Ledger
- `GET {ledger_host}/v2/state/ledger-end` - Get current ledger offset
- `POST {ledger_host}/v2/commands/submit-and-wait-for-transaction-tree` - Execute contract choices
- `POST {ledger_host}/v2/state/active-contracts` - Query active contracts

---

## Key Data Structures

### WithdrawAccount Contract
```
Template: CBTC.WithdrawAccount:CBTCWithdrawAccount
Fields:
  - id: UUID
  - owner: Party (your party ID)
  - operator: Party (attestor's party)
  - registrar: Party (attestor's party)
  - destinationBtcAddress: String (locked at creation)
```

### Holding Contract (CBTC UTXO)
```
Template: Splice.Wallet:Holding
Fields:
  - instrument.id.unpack: String ("CBTC" for CBTC holdings)
  - instrument.version: String
  - amount.quantity: String (decimal amount)
  - owner: Party
  - lock: Optional (null = unlocked, otherwise locked)
```

### WithdrawRequest Contract
```
Template: CBTC.WithdrawRequest:CBTCWithdrawRequest
Fields:
  - id: UUID
  - withdrawAccountId: UUID (links to WithdrawAccount)
  - owner: Party (your party)
  - amount: String (BTC amount in decimal format)
  - destinationBtcAddress: String
  - btcTxId: String or null (null until processed)
  - processed: Bool (false until processed)
```

---

## Notes for Implementers

1. **JWT User ID Extraction**: Must extract the `sub` claim from the JWT and use it as `userId` in Canton submissions. To extract:
   - Split JWT by '.' to get 3 parts
   - Base64-decode the middle part (payload)
   - Parse as JSON
   - Read the `sub` field

2. **UTXO Selection**: Similar to Bitcoin, you must select enough holdings to cover the burn amount. If the sum exceeds the burn amount, Canton will create a change holding.

3. **Token Standard Context**: The `extraArgs.context.values` structure is required by Canton's token standard. Each contract reference uses a specific key (e.g., `utility.digitalasset.com/instrument-configuration`) and is wrapped in a tagged structure (`{"tag": "AV_ContractId", "value": "..."}`).

4. **Decimal Format**: Canton rejects numbers in scientific notation. Always format the `amount` field as a quoted decimal string (e.g., `"0.001"`, not `"1e-3"`).

5. **Disclosed Contracts**: All token standard contracts must be included in the `disclosedContracts` array. This includes burn_mint_factory, instrument_configuration, and any optional contracts that are present.

6. **Locked Holdings**: Filter out holdings where `lock != null`. These are being used in other transactions and cannot be burned.

7. **Polling**: Poll for WithdrawRequest updates every 30-60 seconds. The attestor typically processes withdrawals within minutes.

8. **Destination Address Immutability**: The `destinationBtcAddress` is set when creating the WithdrawAccount and cannot be changed. To withdraw to a different address, create a new WithdrawAccount.

9. **Error Handling**: All endpoints can return errors. Handle HTTP 4xx/5xx status codes and parse error messages from response bodies.

10. **Network-Specific Values**: Template IDs, party IDs, and contract IDs vary by network (devnet/testnet/mainnet). Don't hardcode these values.
