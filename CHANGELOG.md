# Changelog

All notable changes to `cbtc-lib` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-09-09

`cbtc-lib` now re-exports `canton-lib`'s `token` crate for every Token
Standard operation. It deletes its own copies of thirteen modules, 4,731
lines. `mint_redeem` stays, because minting and redeeming are cBTC's own
bridge operations.

### Added

- Token Standard V2 entry points on eight of the thirteen operations —
  `accept`, `batch`, `cancel_offers`, `consolidate`, `distribute`, `reject`,
  `split` and `transfer`, each in a `v2` submodule. `active_contracts`
  reaches V2 through its new `account` field instead. `allocation`,
  `credentials`, `dar_check` and `utils` have no V2 form.
- A caller reaches the V2 API two ways: `cbtc::transfer::v2::submit` beside
  `cbtc::transfer::submit`, or `TokenClientConfig.version`, which selects
  the registry API for every write method and for the holding reads.
  `incoming_offers` and `outgoing_offers` are version-neutral, because the
  registry writes a bare party in `transfer.receiver` under both versions.
- `cbtc::InstrumentId` and `cbtc::Account`, re-exported from `common`. Every
  call site builds an `InstrumentId`, and a consumer that declared its own
  `common` at a different pin would get two `common` packages and two
  incompatible `DamlDecimal` types.
- `cbtc::TokenClient`, a client bound to one token and one party. It stores
  the ledger host, registry URL, instrument, party and credentials that
  otherwise repeat on every call.
- `examples/send_cbtc_v2.rs`, the V2 counterpart of `send_cbtc.rs`.
- `examples/integration_test.rs` reads `TOKEN_STANDARD_VERSION`, `V1` or
  `V2`, and drives the whole flow on either.

### Changed — breaking

- Five parameter structs gained the instrument. The library no longer
  supplies `"CBTC"`, because the team decided on 8 September 2026 that every
  entry point takes the instrument from its caller. Bitsafe plans to support
  other tickers.
  - `active_contracts::Params` gained `instrument_id` and `account`.
  - `consolidate::GetUtxoCountParams` gained `instrument_id` and `account`.
  - `accept::AcceptAllParams` gained `instrument_id`.
  - `cancel_offers::WithdrawAllParams` gained `instrument_id`.
  - `consolidate::CheckConsolidateParams` gained `instrument_id`.
- `utils::fetch_incoming_transfers` and `utils::fetch_outgoing_transfers`
  gained a fourth parameter, `instrument_id`.
- `split::submit` returns `Result<SplitResult, split::Error>` rather than
  `Result<SplitResult, String>`. `Error` carries `message` and `partial`, so
  a caller now learns which holdings the failed split did create. A caller
  that only printed the message reads `e.message`.
- `active_contracts::get` matches the instrument's `id` and `admin`, both
  exactly. It previously kept any holding whose ticker lowercased to
  `"cbtc"`, and never compared the admin. One unsolicited holding with that
  ticker halted every outbound transfer the library attempted, because the
  registry rejects a whole transaction rather than skipping a holding. It
  also inflated the reported balance.
- `utils::fetch_incoming_transfers` and `utils::fetch_outgoing_transfers`
  match the instrument's `id` and `admin`, both exactly. They previously
  kept any offer whose ticker lowercased to `"cbtc"` and never compared the
  admin. **This is the filter that closes the reachable route**: a transfer
  offer names its receiver as an observer only, so any registrar can create
  one at any party, and `accept_all` fed the whole list to the registry.
- The four registry routes report one error wording instead of four. A
  caller that matched on the old per-route text stops matching, and it stops
  silently. I searched for such a caller across `cbtc-lib`, `cbtc-tui`,
  `cbtc-faucet`, `vault-ui`, `cBTC-Canton-App` and `cbtc-doc` on 8 September
  2026 and found none.

### Changed — behaviour

- A split response is parsed by choice name rather than by position.
- A failed split returns the holdings it did create, in `Error.partial`.
- A consolidation reads the holdings once instead of twice. Two reads let a
  holding archive in between, and the mismatch was then reported as an
  account mismatch, which is the wrong diagnosis.
- A registry POST that never reaches the host is retried, on four routes:
  `transfer_factory::get`, `transfer_factory::v2::get`,
  `accept_context::get` and `accept_context::v2::get`. Each attempt waits
  30 seconds and a call makes at most three attempts, so **a wholly
  unreachable registry now fails after about 90 seconds** where it
  previously failed at once or hung forever. A caller with its own timeout
  must check that the timeout exceeds 90 seconds. A response is never
  retried whatever its status: a 4xx or 5xx is the registry's answer, and
  repeating the call would hide it. `allocation_factory::get` and
  `allocation_context::get` do not retry.
- A Keycloak token expiry no longer underflows below a 60-second lifetime.

### Fixed

- **The library could not authenticate against any Keycloak Bitsafe runs.**
  Every call built its token endpoint with `keycloak::login::password_url`,
  deprecated since `canton-lib` 0.5.1, which emits
  `{host}/auth/realms/{realm}/protocol/openid-connect/token`. No Bitsafe
  Keycloak serves the `/auth` prefix: devnet, testnet and mainnet each
  answer 404 on that path and 405 on the path without it. Measured on all
  three hosts on 8 September 2026. Every call moved to
  `keycloak::login::token_url`, which omits the prefix and trims a trailing
  slash from the host. `cbtc-tui` carried the same fault and is fixed with
  it.

### Dependencies

- `token`, `common`, `ledger` and `keycloak` — four crates, not five — pin
  `canton-lib` at revision
  `21ba857c1aa1e1e9955ab72ea46b6cccb6ea5c3f`, the head of
  `feature/token-standard-v2`. **A consumer must pin the same revision.** A
  tag beside a revision makes Cargo build two `common` packages, and then
  `cbtc::DamlDecimal` and `common::decimal::DamlDecimal` are different
  types.
- `registry`, `zip`, `semver`, `base64`, `futures` and `log` are removed.
  Nothing in the crate uses them once the thirteen modules go.
