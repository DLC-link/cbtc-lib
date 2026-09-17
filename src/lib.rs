/// The parameter types the Token Standard operations take.
///
/// `v2::Transfer` cannot sit at the crate root, because `cbtc::transfer` is
/// already `token::transfer`. So a caller reaches it as `cbtc::types::v2`.
///
/// Each name here appears once. The root carries the short names below, and
/// this module carries the rest, so no type is reachable by two paths.
pub mod types {
    pub use common::allocation;
    pub use common::transfer::{DisclosedContract, v2};
    pub use common::transfer_factory;
}
pub use common::decimal::DamlDecimal;
// The short names every call site uses, hoisted to the crate root.
pub use common::instrument::InstrumentId;
pub use common::transfer::{Meta, Transfer, v2::Account};
pub use token::{
    DistributeParams, KeycloakConfig, SendParams, SplitParams, TokenClient, TokenClientConfig,
    TokenStandardVersion, accept, active_contracts, allocation, batch, cancel_offers, consolidate,
    credentials, dar_check, distribute, holding, reject, split, transfer, utils,
};

mod event_helpers;
pub mod mint_redeem;
#[cfg(test)]
mod test_fixtures;
