pub mod attestor;
pub mod constants;
pub mod mint;
pub mod models;
pub mod redeem;

/// Bitsafe's API URL for the `--ignored` live tests in this module tree.
///
/// These tests live in the library, so `examples/shared.rs` does not reach
/// them. Without this they read `BITSAFE_API_URL` alone, and a fresh
/// `cp .env.example .env` stopped them with `BITSAFE_API_URL must be set`.
/// They now follow the same rule the examples do: the variable wins, and
/// `ENVIRONMENT` supplies the value otherwise.
#[cfg(test)]
fn test_api_url() -> String {
    use std::env;

    dotenvy::dotenv().ok();
    resolve_api_url(
        env::var("BITSAFE_API_URL").ok(),
        env::var("ENVIRONMENT").ok(),
    )
}

/// `explicit` when it holds a value, and `environment`'s network value
/// otherwise.
///
/// Both arguments are passed in rather than read here, so the rule is
/// testable without touching the process environment. Environment variables
/// are process-global, and the test harness runs tests in parallel.
///
/// A blank value counts as unset, and both are trimmed. `dotenvy` strips
/// trailing whitespace from a `.env` line, but a shell `export` does not.
#[cfg(test)]
fn resolve_api_url(explicit: Option<String>, environment: Option<String>) -> String {
    fn non_blank(value: Option<String>) -> Option<String> {
        value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    if let Some(value) = non_blank(explicit) {
        return value;
    }
    let name = non_blank(environment)
        .expect("set BITSAFE_API_URL, or set ENVIRONMENT to devnet, testnet or mainnet");
    name.parse::<crate::Network>()
        .unwrap_or_else(|e| panic!("ENVIRONMENT is not a network: {e}"))
        .bitsafe_api_url()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Network;

    #[test]
    fn the_environment_network_supplies_the_api_url_when_the_variable_is_unset() {
        for network in Network::ALL {
            assert_eq!(
                resolve_api_url(None, Some(network.to_string())),
                network.bitsafe_api_url()
            );
        }
    }

    #[test]
    fn an_explicit_variable_wins_over_the_environment_network() {
        assert_eq!(
            resolve_api_url(
                Some("https://api.example".to_string()),
                Some("mainnet".to_string())
            ),
            "https://api.example"
        );
    }

    #[test]
    fn a_blank_variable_counts_as_unset() {
        assert_eq!(
            resolve_api_url(Some("   ".to_string()), Some("testnet".to_string())),
            Network::Testnet.bitsafe_api_url()
        );
    }

    #[test]
    fn both_values_are_trimmed() {
        assert_eq!(
            resolve_api_url(Some("  https://api.example \n".to_string()), None),
            "https://api.example"
        );
        assert_eq!(
            resolve_api_url(None, Some(" mainnet ".to_string())),
            Network::Mainnet.bitsafe_api_url()
        );
    }

    #[test]
    #[should_panic(expected = "set BITSAFE_API_URL")]
    fn neither_value_panics_and_names_both_variables() {
        resolve_api_url(None, None);
    }

    #[test]
    #[should_panic(expected = "ENVIRONMENT is not a network")]
    fn an_unknown_environment_panics() {
        resolve_api_url(None, Some("staging".to_string()));
    }
}
