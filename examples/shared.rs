#![allow(dead_code)]
//! The per-network values the examples need.
//!
//! `mod shared;` makes this a private module of each example crate, so a
//! function that example does not call is dead code in that crate. Most
//! examples need fewer than three, hence the attribute above.

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
/// The variable wins, so an existing `.env` keeps working and a moved URL
/// stays a one-line fix rather than a release and a repin.
///
/// A blank value counts as unset. An empty party ID would otherwise reach
/// the ledger, which accepts it, and no holding matches.
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
/// The precedence rule works one variable at a time, so a `.env` can name
/// two networks at once and resolve without complaint. A wrong registrar
/// then yields a zero balance, and a wrong API URL reaches the wrong
/// service. Neither raises an error.
///
/// It fires only when the override holds another named network's value, so
/// a custom deployment stays silent.
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
/// `dotenvy` strips trailing whitespace from a `.env` line, but a shell
/// `export` does not, and a party ID with a trailing space matches no
/// holding.
fn non_blank(variable: &str) -> Option<String> {
    env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
