// Core modules available on all targets (including WASM)
pub mod accept;
pub mod active_contracts;
pub mod cancel_offers;
pub mod consolidate;
pub mod distribute;
pub mod mint_redeem;
pub mod split;
pub mod transfer;
pub mod utils;

// Modules that require file I/O - only available on native targets
#[cfg(not(target_arch = "wasm32"))]
pub mod batch;
