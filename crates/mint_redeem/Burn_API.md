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

**Example Request:**
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
- `wa_rules` = WithdrawAccountRules contract that governs withdraw account creation
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

**Example Request:**
```json
{
  "commands": [
    {
      "ExerciseCommand": {
        "templateId": "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
        "contractId": "00c007c8b886320cf18412333c614bc5a0b098015992aadc8387a6ed60139c730...",
        "choice": "CBTCWithdrawAccountRules_CreateWithdrawAccount",
        "choiceArgument": {
          "owner": "your-party::1220abcdef...",
          "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3"
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
      "templateId": "43a8452a56388d22c6058abe03e90dadbae9a20a682634568b07a93531dda1a3:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
      "contractId": "00c007c8b886320cf18412333c614bc5a0b098015992aadc8387a6ed60139c730...",
      "createdEventBlob": "CgMyLjES/wQKRQDAB8i4hjIM8YQSMzxhS8WgsJgBWZKq3IOHpu1gE5xzBco...",
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
            "contractId": "00c007c8b886320cf18412333c614bc5a0b098015992aadc8387a6ed60139c730...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawAccount:CBTCWithdrawAccountRules",
            "choice": "CBTCWithdrawAccountRules_CreateWithdrawAccount",
            "choiceArgument": {
              "owner": "your-party::1220abcdef...",
              "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3"
            },
            "actingParties": ["your-party::1220abcdef..."],
            "consuming": false,
            "exerciseResult": {
              "withdrawAccountCid": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85..."
            },
            "packageName": "cbtc"
          }
        }
      },
      "1": {
        "CreatedTreeEvent": {
          "value": {
            "contractId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawAccount:CBTCWithdrawAccount",
            "createArgument": {
              "registrar": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
              "operator": "operator-party::1220...",
              "instrument": {
                "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
                "id": "CBTC"
              },
              "owner": "your-party::1220abcdef...",
              "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3"
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
- `templateId` in `ExerciseCommand` uses shorthand format `#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccountRules`
- `choice` is `CBTCWithdrawAccountRules_CreateWithdrawAccount` (includes the template name prefix)
- `actAs`: The party creating the account (must match authenticated user's party)
- `commandId`: Unique identifier for idempotency (use UUID v4)
- `disclosedContracts`: Must include the full WithdrawAccountRules contract from step 2
- `disclosedContracts[].templateId` uses full hash format (different from ExerciseCommand templateId)
- `disclosedContracts[].synchronizerId` should be an empty string
- `destinationBtcAddress`: The Bitcoin address where BTC will be sent (locked at creation time and cannot be changed)

**Response:**
- Contains two events in `eventsById`: `ExercisedTreeEvent` (index "0") and `CreatedTreeEvent` (index "1")
- The `ExercisedTreeEvent` shows which choice was exercised on the rules contract
- The `CreatedTreeEvent` contains the actual WithdrawAccount contract that was created
- `createArgument.instrument` contains the token information with `id: "CBTC"`
- The created WithdrawAccount contract ID is used in step 7 to burn CBTC

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

**Example Response:**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00holding123...",
          "templateId": "...(implementation-specific template that implements HoldingV1 interface)...",
          "createArgument": {
            "instrument": {
              "id": "CBTC",
              "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff"
            },
            "amount": "0.001",
            "owner": "your-party::1220abcdef...",
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
          "templateId": "...(implementation-specific template that implements HoldingV1 interface)...",
          "createArgument": {
            "instrument": {
              "id": "CBTC",
              "admin": "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff"
            },
            "amount": "0.002",
            "owner": "your-party::1220abcdef...",
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
- **IMPORTANT**: There is no `Splice.Wallet:Holding` template - holdings are accessed via the `#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding` interface
- Uses `InterfaceFilter` (not `TemplateIdentifierFilter`) to query via Canton's interface system
- `interfaceId` uses shorthand format: `#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding`
- `includeInterfaceView: true` includes the interface view in the response
- `includeCreatedEventBlob: true` includes the blob for disclosed contracts
- `verbose: true` returns full contract details
- `activeAtOffset` requires getting the ledger end offset first (step 3)
- Holdings are UTXO-based like Bitcoin - each holding is a separate contract
- The actual template ID in responses will vary by implementation, but all holdings expose the standard `HoldingV1` interface
- Filter for `instrument.id == "CBTC"` and `owner == your_party`
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

**Example Request:**
```json
{
  "commands": [
    {
      "ExerciseCommand": {
        "templateId": "#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccount",
        "contractId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
        "choice": "CBTCWithdrawAccount_Withdraw",
        "choiceArgument": {
          "amount": "0.001",
          "tokens": [
            "00holding123...",
            "00holding456..."
          ],
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
  ],
  "actAs": [
    "your-party::1220abcdef..."
  ],
  "commandId": "cmd-550e8400-e29b-41d4-a716-446655440001",
  "disclosedContracts": [
    {
      "templateId": "...Splice.TokenStandard:BurnMintFactory",
      "contractId": "00factory123...",
      "createdEventBlob": "base64-encoded-burn-mint-factory...",
      "synchronizerId": ""
    },
    {
      "templateId": "...Splice.TokenStandard:InstrumentConfiguration",
      "contractId": "00instrument123...",
      "createdEventBlob": "base64-encoded-instrument-configuration...",
      "synchronizerId": ""
    },
    {
      "templateId": "...Splice.TokenStandard:IssuerCredential",
      "contractId": "00issuer123...",
      "createdEventBlob": "base64-encoded-issuer-credential...",
      "synchronizerId": ""
    }
  ]
}
```

**Example Response:**
```json
{
  "transactionTree": {
    "updateId": "1220abc456...",
    "commandId": "cmd-550e8400-e29b-41d4-a716-446655440001",
    "workflowId": "",
    "effectiveAt": "2025-11-05T10:30:15.123456Z",
    "offset": 3896234,
    "eventsById": {
      "0": {
        "ExercisedTreeEvent": {
          "value": {
            "contractId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawAccount:CBTCWithdrawAccount",
            "choice": "CBTCWithdrawAccount_Withdraw",
            "choiceArgument": {
              "amount": "0.001",
              "tokens": [
                "00holding123...",
                "00holding456..."
              ],
              "burnMintFactoryCid": "00factory123...",
              "extraArgs": { }
            },
            "actingParties": ["your-party::1220abcdef..."],
            "consuming": false,
            "exerciseResult": {
              "withdrawRequestCid": "00withdraw-request-123..."
            },
            "packageName": "cbtc"
          }
        }
      },
      "1": {
        "CreatedTreeEvent": {
          "value": {
            "contractId": "00withdraw-request-123...",
            "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawRequest:CBTCWithdrawRequest",
            "createArgument": {
              "withdrawAccountId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
              "owner": "your-party::1220abcdef...",
              "amount": "0.001",
              "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
              "btcTxId": null
            },
            "createdEventBlob": "",
            "witnessParties": ["your-party::1220abcdef..."],
            "signatories": [
              "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff",
              "your-party::1220abcdef..."
            ],
            "observers": [],
            "createdAt": "2025-11-05T10:30:15.123456Z",
            "packageName": "cbtc"
          }
        }
      },
      "2": {
        "ExercisedTreeEvent": {
          "value": {
            "contractId": "00holding123...",
            "templateId": "...(implementation-specific holding template)...",
            "choice": "Archive",
            "consuming": true
          }
        }
      },
      "3": {
        "ExercisedTreeEvent": {
          "value": {
            "contractId": "00holding456...",
            "templateId": "...(implementation-specific holding template)...",
            "choice": "Archive",
            "consuming": true
          }
        }
      }
    },
    "synchronizerId": "global-domain::1220...",
    "recordTime": "2025-11-05T10:30:15.789012Z"
  }
}
```

**Notes:**

**Request:**
- `templateId` in `ExerciseCommand` uses shorthand format `#cbtc:CBTC.WithdrawAccount:CBTCWithdrawAccount`
- `choice` is `CBTCWithdrawAccount_Withdraw` (includes the template name prefix)
- `contractId`: The WithdrawAccount contract ID from step 4
- `amount`: The amount of BTC to withdraw (must be <= sum of holding amounts)
- `tokens`: Array of holding contract IDs to burn (must have sufficient total amount)
- `burnMintFactoryCid`: Optional burn_mint_factory contract ID from step 6 (can be null)
- `extraArgs`: Optional token standard context with all required contract references
  - `context.values["utility.digitalasset.com/instrument-configuration"]`: Always required
  - `context.values["utility.digitalasset.com/issuer-credentials"]`: List of issuer credentials (if present)
  - `context.values["utility.digitalasset.com/app-reward-configuration"]`: App reward config (if present)
  - `context.values["utility.digitalasset.com/featured-app-right"]`: Featured app right (if present)
  - `meta.values`: Metadata about the operation (optional)
- **IMPORTANT**: The `amount` field must be a quoted decimal string (not scientific notation) or Canton will reject it
- `disclosedContracts`: Must include all token standard contracts from step 6 (burn_mint_factory, instrument_configuration, and any optional ones)
- `disclosedContracts[].templateId` uses full hash format (different from ExerciseCommand templateId)
- `actAs`: The party burning CBTC (must match authenticated user's party)
- `commandId`: Unique identifier for idempotency (use UUID v4)

**Response:**
- Contains multiple events in `eventsById`:
  - `ExercisedTreeEvent` (index "0"): Shows the Withdraw choice being exercised
  - `CreatedTreeEvent` (index "1"): The created WithdrawRequest contract (with `btcTxId: null` initially)
  - Additional `ExercisedTreeEvent`s: The archived (burned) holding contracts
- If the sum of selected holdings exceeds the burn amount, Canton will create a change holding (not shown in this example)
- The attestor network monitors for new WithdrawRequest contracts and processes the BTC withdrawal

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

**Example Request:**
```json
{
  "verbose": true,
  "activeAtOffset": 3896235,
  "filter": {
    "filtersByParty": {
      "your-party::1220abcdef...": {
        "cumulative": [
          {
            "identifierFilter": {
              "TemplateIdentifierFilter": {
                "value": {
                  "templateId": "#cbtc:CBTC.WithdrawRequest:CBTCWithdrawRequest",
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

**Example Response (initially):**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00withdraw-request-123...",
          "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawRequest:CBTCWithdrawRequest",
          "createArgument": {
            "withdrawAccountId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
            "owner": "your-party::1220abcdef...",
            "amount": "0.001",
            "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
            "btcTxId": null
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

**Example Response (after attestor processing):**
```json
[
  {
    "contractEntry": {
      "JsActiveContract": {
        "createdEvent": {
          "contractId": "00withdraw-request-123...",
          "templateId": "61ed690af72fda469c2a2df960d81bf59be5ff8d0f4844e816944b5fce267d92:CBTC.WithdrawRequest:CBTCWithdrawRequest",
          "createArgument": {
            "withdrawAccountId": "0056b9c28cb6cd0e7c5a75e554aaced4e7312713862a72a25ed55f3e124e89c85...",
            "owner": "your-party::1220abcdef...",
            "amount": "0.001",
            "destinationBtcAddress": "bc1q5j3x2h9z8y7k6m4n3p2r1s0t9u8v7w6x5y4z3",
            "btcTxId": "abc123def456..."
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
- `templateId` can use shorthand format `#cbtc:CBTC.WithdrawRequest:CBTCWithdrawRequest` in the query
- Initially `btcTxId` will be `null`
- Once the attestor processes the withdrawal:
  - `btcTxId` is set to the Bitcoin transaction ID
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
  - owner: Party (your party ID)
  - operator: Party (attestor's party)
  - registrar: Party (attestor's party)
  - instrument: Object
    - id: String (e.g., "CBTC")
    - admin: Party (CBTC network party)
  - destinationBtcAddress: String (locked at creation and cannot be changed)
```

### Holding Contract (CBTC UTXO)
```
Interface: #splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding
Note: There is no specific "Holding" template - various templates implement this interface
Fields (via HoldingV1 interface):
  - instrument: Object
    - id: String (e.g., "CBTC")
    - admin: Party (CBTC network party)
  - amount: String (decimal amount of CBTC)
  - owner: Party (token owner)
  - lock: Optional (null = unlocked, otherwise locked in a transaction)
```

**Important**: Holdings are queried using `InterfaceFilter` with the `HoldingV1` interface, not by a specific template. The actual template ID will vary depending on the implementation, but all holdings expose the standard interface fields listed above.

### WithdrawRequest Contract
```
Template: CBTC.WithdrawRequest:CBTCWithdrawRequest
Fields:
  - withdrawAccountId: String (contract ID of the WithdrawAccount)
  - owner: Party (your party)
  - amount: String (BTC amount in decimal format)
  - destinationBtcAddress: String (where BTC will be sent)
  - btcTxId: String or null (null until processed by attestor)
```

**Note**: Once the attestor processes the withdrawal, the `btcTxId` field is updated with the Bitcoin transaction ID.

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
