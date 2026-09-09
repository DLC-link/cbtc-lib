/// The parameter types the Token Standard operations take.
///
/// Naming the individual types at the crate root is not enough:
/// `transfer::Params.transfer` is a `common::transfer::Transfer` and
/// `transfer::v2::Params.transfer` is a `common::transfer::v2::Transfer`, so a
/// consumer needs the module to call the library's main operation at all. The
/// V2 type cannot sit at the crate root, because `cbtc::transfer` is already
/// `token::transfer`.
///
/// This module carries only what those signatures name. `cbtc` does not
/// re-export `common` whole, so a `common` change reaches this crate's public
/// API only where a signature already used it.
pub mod types {
    pub use common::allocation;
    pub use common::transfer::*;
}
pub use common::decimal::DamlDecimal;
// The three types every call site names, hoisted to the crate root.
pub use common::transfer::{InstrumentId, Meta, Transfer, v2::Account};
pub use token::{
    DistributeParams, KeycloakConfig, SendParams, SplitParams, TokenClient, TokenClientConfig,
    TokenStandardVersion, accept, active_contracts, allocation, batch, cancel_offers, consolidate,
    credentials, dar_check, distribute, reject, split, transfer, utils,
};

mod event_helpers;
pub mod mint_redeem;
#[cfg(test)]
mod test_fixtures;
