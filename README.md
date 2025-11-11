# Canton CBTC Token Library

A Rust library for interacting with the Canton blockchain to manage CBTC (Canton Bitcoin) tokens using the Canton Token Standard (CIP-0056).

## Features

- ✅ **Send CBTC** - Transfer tokens to other parties
- ✅ **Accept CBTC** - Accept incoming transfers as a receiver
- ✅ **Batch Distribution** - Efficiently distribute tokens to multiple recipients
- ✅ **UTXO Management** - Consolidate and split holdings
- ✅ **Reward Farming** - Optimized for high-volume transfer operations
- ✅ **Multi-Environment** - Support for devnet, testnet, and mainnet
- ✅ **Token Standard Compliant** - Implements Canton Token Standard (CIP-0056)

---

## Table of Contents

1. [Quick Start - How to Use This Library](#quick-start---how-to-use-this-library)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Usage Examples](#usage-examples)
   - [Sending CBTC](#sending-cbtc)
   - [Accepting CBTC](#accepting-cbtc)
   - [Batch Distribution](#batch-distribution)
   - [UTXO Management](#utxo-management)
5. [High-Volume Operations](#high-volume-operations)
6. [API Reference](#api-reference)
7. [Direct Canton API Usage](#direct-canton-api-usage-reference)
8. [Testing](#testing)
9. [Contributing](#contributing)

---

## Quick Start - How to Use This Library

This library provides a high-level Rust interface for interacting with CBTC (Canton Bitcoin) tokens on the Canton blockchain. Here's how to get started:

### Prerequisites

Before using this library, you need:
1. **A Canton Participant Node** - Access to a Canton participant node (devnet, testnet, or mainnet)
2. **Keycloak Credentials** - Authentication credentials for your participant node
3. **A Party ID** - Your unique party identifier on the Canton network
4. **CBTC Holdings** - Some CBTC tokens in your account (for sending/distributing)

### Three Ways to Use This Library

#### 1. Run the Examples (Fastest Way to Start)

The quickest way to see the library in action:

```bash
# Clone the repository
git clone <your-repo-url>
cd cbtc-lib

# Set up your environment
cp .env.example .env
# Edit .env with your Canton credentials

# Run an example
cargo run -p examples --example check_balance
cargo run -p examples --example send_cbtc
```

See [Quick Start with Examples](#quick-start-with-examples) for more details.

#### 2. Use as a Library in Your Project

Add to your `Cargo.toml`:
```toml
[dependencies]
cbtc = { path = "path/to/cbtc-lib/crates/cbtc" }
ledger = { path = "path/to/cbtc-lib/crates/ledger" }
registry = { path = "path/to/cbtc-lib/crates/registry" }
common = { path = "path/to/cbtc-lib/crates/common" }
keycloak = { path = "path/to/cbtc-lib/crates/keycloak" }
```

Then in your code:
```rust
use cbtc::transfer;
use keycloak::login;

// Authenticate
let auth = login::password(login::PasswordParams {
    client_id: "your-client-id".to_string(),
    username: "your-username".to_string(),
    password: "your-password".to_string(),
    url: login::password_url("https://your-keycloak-host", "your-realm"),
}).await?;

// Send CBTC
transfer::submit(transfer::Params {
    // ... see Usage Examples section
}).await?;
```

#### 3. Understand the Low-Level API

For advanced users who want direct control, see [Direct Canton API Usage](#direct-canton-api-usage-reference) to learn how to interact with Canton's REST APIs directly.

### Common Operations

| Task | Function | Section |
|------|----------|---------|
| Check balance | `cbtc::active_contracts::get()` | [UTXO Management](#utxo-management) |
| Send tokens | `cbtc::transfer::submit()` | [Sending CBTC](#sending-cbtc) |
| Accept tokens | `cbtc::accept::submit()` | [Accepting CBTC](#accepting-cbtc) |
| Batch send | `cbtc::batch::submit_from_csv()` | [Batch Distribution](#batch-distribution) |
| Consolidate UTXOs | `cbtc::consolidate::check_and_consolidate()` | [UTXO Management](#utxo-management) |

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cbtc = { path = "path/to/cbtc-lib/crates/cbtc" }
ledger = { path = "path/to/cbtc-lib/crates/ledger" }
registry = { path = "path/to/cbtc-lib/crates/registry" }
common = { path = "path/to/cbtc-lib/crates/common" }
keycloak = { path = "path/to/cbtc-lib/crates/keycloak" }
```

---

## Configuration

### Setup

1. **Create environment configuration**

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
```

2. **Configure your environment variables**

Edit `.env` with your Canton participant node details:

```bash
# Authentication
KEYCLOAK_HOST=https://keycloak.example.com
KEYCLOAK_REALM=your-realm
KEYCLOAK_CLIENT_ID=your-client-id
KEYCLOAK_USERNAME=your-username
KEYCLOAK_PASSWORD=your-password

# Canton Network
LEDGER_HOST=https://participant.example.com
PARTY_ID=your-party::1220...
DECENTRALIZED_PARTY_ID=cbtc-network::1220...  # See environment-specific values below
REGISTRY_URL=https://api.utilities.digitalasset-dev.com  # See environment-specific values below
```

### Environment-Specific Values

#### Devnet
```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff
REGISTRY_URL=https://api.utilities.digitalasset-dev.com
```

#### Testnet
```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f
REGISTRY_URL=https://api.utilities.digitalasset-staging.com
```

#### Mainnet
```bash
DECENTRALIZED_PARTY_ID=cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262
REGISTRY_URL=https://api.utilities.digitalasset.com
```

---

## Quick Start with Examples

For quick experimentation, this library includes ready-to-run example programs. See the [`crates/examples`](crates/examples/README.md) directory for:

- `check_balance` - Check your CBTC balance and UTXO count
- `send_cbtc` - Send tokens to another party
- `accept_transfers` - Accept all pending incoming transfers
- `consolidate_utxos` - Consolidate multiple UTXOs
- `batch_distribute` - Distribute tokens to multiple recipients from a CSV file

Run examples from the workspace root:
```bash
cargo run -p examples --example check_balance
```

See the [examples README](crates/examples/README.md) for detailed instructions.

---

## Usage Examples

This library provides several high-level operations for working with CBTC tokens. Below is a quick reference - for complete working examples, see the [`crates/examples`](crates/examples) directory.

### Core Operations

| Operation | Example File | Description |
|-----------|-------------|-------------|
| **Check Balance** | [`check_balance.rs`](crates/examples/examples/check_balance.rs) | View your CBTC balance and UTXO count |
| **Send CBTC** | [`send_cbtc.rs`](crates/examples/examples/send_cbtc.rs) | Transfer tokens to another party |
| **Accept CBTC** | [`accept_transfers.rs`](crates/examples/examples/accept_transfers.rs) | Accept incoming transfers |
| **Batch Distribution** | [`batch_distribute.rs`](crates/examples/examples/batch_distribute.rs) | Distribute to multiple recipients from CSV |
| **Consolidate UTXOs** | [`consolidate_utxos.rs`](crates/examples/examples/consolidate_utxos.rs) | Merge multiple UTXOs into one |

### Key Concepts

**Authentication**: All operations require Keycloak/OIDC authentication. The library handles token management - you just provide credentials.

**UTXO Model**: CBTC uses a UTXO (Unspent Transaction Output) model similar to Bitcoin. Each holding is a separate UTXO that can be split or combined.

**Two-Phase Transfers**:
1. Sender creates a transfer offer
2. Receiver must accept the transfer to complete it

See the [examples README](crates/examples/README.md) for detailed usage instructions.

### Understanding UTXO Management

**What are UTXOs?**

Every CBTC holding is a UTXO (Unspent Transaction Output), similar to Bitcoin. Each transfer can create new UTXOs, and over time you may accumulate many small ones.

**Why Consolidate?**
- **Performance**: Canton has a soft limit of **10 UTXOs per party** per token type
- **Node Efficiency**: Fewer UTXOs reduce database and memory usage
- **Network Load**: Smaller transactions with fewer inputs

**Best Practice**: Consolidate regularly, especially for high-volume operations. See [`consolidate_utxos.rs`](crates/examples/examples/consolidate_utxos.rs) for example code.

---

## High-Volume Operations

For applications running high-volume CBTC transfers (e.g., reward farming, payment processors):

### Best Practices

1. **Monitor UTXOs**: Consolidate when approaching 10 UTXOs per party
2. **Use Batch Operations**: `batch::submit_from_csv()` for efficient multi-recipient transfers
3. **Consolidate Proactively**: Check and consolidate before large distributions
4. **Handle Both Parties**: If you control sender and receiver, consolidate both

### Recommended Workflow

```bash
# 1. Check UTXO count and consolidate if needed
cargo run -p examples --example consolidate_utxos

# 2. Run batch distribution
cargo run -p examples --example batch_distribute
```

See [`batch_distribute.rs`](crates/examples/examples/batch_distribute.rs) and [`batch_with_callback.rs`](crates/examples/examples/batch_with_callback.rs) for complete examples with callbacks and logging.

---

## API Reference

### Core Modules

#### `cbtc::transfer`
- `submit(Params)` - Send CBTC to a single recipient
- `submit_multi(MultiParams)` - Send CBTC to multiple recipients in one transaction

#### `cbtc::accept`
- `submit(Params)` - Accept an incoming CBTC transfer

#### `cbtc::distribute`
- `submit(Params)` - Distribute CBTC to multiple recipients

#### `cbtc::batch`
- `submit_from_csv(Params)` - Batch distribution from CSV file

#### `cbtc::consolidate`
- `check_and_consolidate(CheckConsolidateParams)` - Check and consolidate if needed
- `get_utxo_count(GetUtxoCountParams)` - Get UTXO count
- `consolidate_utxos(ConsolidateParams)` - Force consolidation

#### `cbtc::split`
- `submit(Params)` - Split holdings into specific amounts

#### `cbtc::active_contracts`
- `get(Params)` - Get active CBTC holdings

### Helper Modules

#### `keycloak::login`
- `password(PasswordParams)` - Authenticate with username/password
- `client_credentials(ClientCredentialsParams)` - Service account authentication

#### `ledger`
- Low-level ledger API operations
- WebSocket streaming for real-time updates

#### `registry`
- Registry service integration
- Factory contract queries

---

## Direct Canton API Usage (Reference)

For teams who want to understand the underlying protocol or implement custom workflows, here's how to interact with Canton APIs directly.

### Prerequisites

- Access to a Canton participant node
- Valid OIDC authentication token
- Understanding of Canton's UTXO model

### Get Active Contracts

```bash
LEDGER_OFFSET=$(curl -X GET "$LEDGER_HOST/v2/state/ledger-end" \
  -H "Authorization: Bearer $ACCESS_TOKEN" | jq -r '.offset')

curl -X POST $LEDGER_HOST/v2/state/active-contracts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -d '{
    "filter": {
      "filtersByParty": {
        "$SENDER_PARTY_ID": {
          "cumulative": [{
            "identifierFilter": {
              "InterfaceFilter": {
                "value": {
                  "interfaceId": "#splice-api-token-holding-v1:Splice.Api.Token.HoldingV1:Holding",
                  "includeInterfaceView": true,
                  "includeCreatedEventBlob": true
                }
              }
            }
          }]
        }
      }
    },
    "verbose": false,
    "activeAtOffset": '$(echo $LEDGER_OFFSET | jq -R 'tonumber')'
  }' | jq
```

### Get Factory Disclosures

```bash
curl -X POST $REGISTRY_URL/api/token-standard/v0/registrars/$DECENTRALIZED_PARTY_ID/registry/transfer-instruction/v1/transfer-factory \
  -H "Content-Type: application/json" \
  -d '{
    "choiceArguments": {
      "expectedAdmin": "'$DECENTRALIZED_PARTY_ID'",
      "transfer": {
        "sender": "'$SENDER_PARTY_ID'",
        "receiver": "'$RECEIVER_PARTY_ID'",
        "amount": 0.5,
        "instrumentId": {
          "admin": "'$DECENTRALIZED_PARTY_ID'",
          "id": "CBTC"
        },
        "requestedAt": "'$(date -u +"%Y-%m-%dT%H:%M:%SZ")'",
        "executeBefore": "'$(date -u -d "+1 days" +"%Y-%m-%dT%H:%M:%SZ")'",
        "inputHoldingCids": ["'$HOLDING_CID'"]
      },
      "extraArgs": {
        "context": {"values": {}},
        "meta": {"values": {}}
      }
    },
    "excludeDebugFields": true
  }' | jq
```

### Submit Transfer

See [example_transfer.sh](example_transfer.sh) for a complete example.

### Accept Transfer

```bash
# Get accept context
curl -X POST $REGISTRY_URL/api/token-standard/v0/registrars/$DECENTRALIZED_PARTY_ID/registry/transfer-instruction/v1/$TRANSFER_OFFER_CID/choice-contexts/accept \
  -H "Content-Type: application/json" \
  -d '{"meta":{}}' | jq

# Submit acceptance (use disclosed contracts from above)
curl -X POST $LEDGER_HOST/v2/commands/submit-and-wait-for-transaction-tree \
  -H "Authorization: Bearer $RECEIVER_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "commands": [{
      "ExerciseCommand": {
        "templateId": "#splice-api-token-transfer-instruction-v1:Splice.Api.Token.TransferInstructionV1:TransferInstruction",
        "contractId": "'$TRANSFER_OFFER_CID'",
        "choice": "TransferInstruction_Accept",
        "choiceArgument": {
          "extraArgs": {
            "context": {"values": '$CHOICE_CONTEXT_VALUES'},
            "meta": {"values": {}}
          }
        }
      }
    }],
    "commandId": "'$(uuidgen)'",
    "actAs": ["'$RECEIVER_PARTY'"],
    "disclosedContracts": '$DISCLOSED_CONTRACTS'
  }' | jq
```

---

## Contributing

We welcome contributions from the Canton ecosystem! This library is designed to help developers build on Canton's CBTC token standard.

### How to Contribute

#### Reporting Issues

Found a bug or have a feature request?
1. Check [existing issues](../../issues) to avoid duplicates
2. Open a new issue with:
   - Clear description of the problem or feature
   - Steps to reproduce (for bugs)
   - Expected vs actual behavior
   - Your environment (Canton network, Rust version, OS)

#### Contributing Code

1. **Fork the repository** and create a feature branch
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow Rust best practices and naming conventions
   - Keep library code free of environment variable dependencies
   - Add tests for new functionality (when applicable)
   - Update documentation and examples

3. **Test your changes**
   ```bash
   # Build the library
   cargo build --release

   # Build examples
   cargo build -p examples --release

   # Run clippy for linting
   cargo clippy --all-targets --all-features

   # Format code
   cargo fmt --all
   ```

4. **Submit a pull request**
   - Provide a clear description of your changes
   - Reference any related issues
   - Ensure CI checks pass

### Development Guidelines

#### Code Organization

- **Library code** (`crates/cbtc`, `crates/ledger`, etc.) should:
  - Accept all configuration as function parameters (no `env::var()` calls)
  - Be environment-agnostic and testable
  - Follow dependency injection patterns

- **Example code** (`crates/examples`) can:
  - Read from environment variables
  - Demonstrate practical usage patterns
  - Show best practices for error handling

#### Testing

- **Integration tests** require live Canton network access and credentials
- Set required environment variables (see `.env.example`)
- Tests expect to connect to real endpoints - they will fail without proper setup
- This is intentional: tests validate real-world behavior

#### Documentation

- Add doc comments to public functions using `///`
- Include usage examples in doc comments
- Update README.md for significant changes
- Keep examples up-to-date with API changes

### Getting Help

- Review the [examples](crates/examples) directory for usage patterns
- Check the [Canton documentation](https://docs.digitalasset.com/canton) for protocol details
- Open a discussion for questions about the library
- Join the Canton community for broader ecosystem questions

### License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Testing

This library includes integration tests that validate real-world interactions with Canton networks.

### Running Tests

Tests require:
- Access to a Canton participant node
- Valid Keycloak credentials
- Network connectivity

Set up your environment:
```bash
cp .env.example .env
# Edit .env with your Canton credentials
```

Run tests:
```bash
# Build the library (always works)
cargo build --release

# Run integration tests (requires credentials)
cargo test --lib

# Note: Tests will fail without proper environment variables
# This is expected and intentional - they validate real network behavior
```

### Why Tests Require Credentials

Unlike unit tests, these are **integration tests** that:
- Connect to actual Canton participant nodes
- Perform real ledger operations
- Validate end-to-end workflows

This ensures the library works correctly with real Canton infrastructure, not just in isolation.

---

## License

MIT License - see [LICENSE](LICENSE) file for details

## Resources

- [Canton Token Standard (CIP-0056)](context/cip-0056.md)
- [Canton Coin Fee Removal (CIP-0078)](context/cip-0078-canton-coin-fee-removal.md)
- [Canton Documentation](https://docs.digitalasset.com/canton)
- [Canton Network](https://www.canton.network/)
