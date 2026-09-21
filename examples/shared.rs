#![allow(dead_code)]
//! The per-network values the examples need.
//!
//! `mod shared;` makes this a private module of each example crate, so a
//! function that example does not call is dead code *in that crate*.
//! Seventeen of the eighteen adopters use fewer than three values —
//! `integration_test` uses all three, twelve use two and five use one —
//! so seventeen would warn without the attribute above, and
//! `cargo clippy --workspace --all-targets` could not come back clean.

use std::env;

use cbtc::Network;

/// The CBTC registrar, from `DECENTRALIZED_PARTY_ID` or the
/// `ENVIRONMENT` network.
pub fn resolve_party_id() -> String {
    resolve("DECENTRALIZED_PARTY_ID", Network::decentralized_party_id)
}

/// Digital Asset's utility registry, from `REGISTRY_URL` or the
/// `ENVIRONMENT` network.
pub fn resolve_registry_url() -> String {
    resolve("REGISTRY_URL", Network::registry_url)
}

/// Bitsafe's API, from `BITSAFE_API_URL` or the `ENVIRONMENT` network.
pub fn resolve_bitsafe_api_url() -> String {
    resolve("BITSAFE_API_URL", Network::bitsafe_api_url)
}

/// `variable`'s value when it holds one, and `ENVIRONMENT`'s network
/// value otherwise.
///
/// The variable wins for two reasons. It keeps every existing `.env`
/// working unchanged. And it stops a moved URL from blocking anyone: on
/// 9 April 2026 the mainnet Bitsafe endpoint moved, and with this order
/// that stays a one-line `.env` fix rather than a release, a tag and a
/// repin in five repositories.
///
/// A blank value counts as unset. `DECENTRALIZED_PARTY_ID=` in a copied
/// template must fall through to the network rather than yield an empty
/// party ID, which the ledger accepts and no holding matches.
///
/// Call this after `dotenvy::dotenv()`. Before it, `.env` has not
/// reached the process environment, so every variable reads as unset and
/// this falls through to the network without saying so.
fn resolve(variable: &str, from_network: fn(Network) -> &'static str) -> String {
    if let Some(value) = non_blank(variable) {
        if let Some(warning) = cross_network_warning(variable, &value) {
            eprintln!("{warning}");
        }
        return value;
    }
    let name = non_blank("ENVIRONMENT").unwrap_or_else(|| {
        panic!("set {variable}, or set ENVIRONMENT to devnet, testnet or mainnet")
    });
    let network: Network = name
        .parse()
        .unwrap_or_else(|e| panic!("ENVIRONMENT is not a network: {e}"));
    from_network(network).to_string()
}

/// A warning when `value` is another named network's value, and
/// `ENVIRONMENT` names a different one.
///
/// The precedence rule works one variable at a time, so a `.env` can name two
/// networks at once and resolve without complaint. Two ways in: a user sets
/// `ENVIRONMENT=mainnet` on an existing devnet `.env`, and the stale party ID
/// still wins; or a user sets the party ID and registry URL to mainnet,
/// forgets the API URL, and leaves `ENVIRONMENT=devnet`. Both read a wrong
/// value with no error, because a wrong registrar yields a zero balance and a
/// wrong API URL reaches the wrong service.
///
/// This fires only when the override holds another *named* network's value.
/// A custom deployment's own URL matches none of the nine, so a deliberate
/// override stays silent.
pub fn cross_network_warning(variable: &str, value: &str) -> Option<String> {
    let chosen: Network = non_blank("ENVIRONMENT")?.parse().ok()?;
    let named = Network::ALL.into_iter().find(|network| {
        network.decentralized_party_id() == value
            || network.registry_url() == value
            || network.bitsafe_api_url() == value
    })?;
    if named == chosen {
        return None;
    }
    Some(format!(
        "warning: {variable} holds {named}'s value, but ENVIRONMENT is {chosen}. \
         The explicit variable wins, so this run mixes two networks."
    ))
}

/// `variable`'s value, trimmed, treating whitespace and the empty string as
/// unset.
///
/// It returns the trimmed value, not the raw one. `dotenvy` strips trailing
/// whitespace from a `.env` line, but a shell `export` does not, and a party
/// ID with a trailing space matches no holding. `cbtc-tui`'s `parse_env`
/// trims, so this keeps the two paths in agreement.
fn non_blank(variable: &str) -> Option<String> {
    env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
