pub use common::decimal::DamlDecimal;
// Every call site builds an InstrumentId, and a V2 caller builds an
// Account. Neither `token` nor `common` is a dependency a consumer of
// this crate declares, and a consumer that added `common` at a different
// pin would get two `common` packages and two incompatible `DamlDecimal`
// types. So re-export both (decision 18).
pub use common::transfer::{InstrumentId, v2::Account};
pub use token::{
    DistributeParams, KeycloakConfig, SendParams, SplitParams, TokenClient, TokenClientConfig,
    TokenStandardVersion, accept, active_contracts, allocation, batch, cancel_offers, consolidate,
    credentials, dar_check, distribute, reject, split, transfer, utils,
};

mod event_helpers;
pub mod mint_redeem;
#[cfg(test)]
mod test_fixtures;
