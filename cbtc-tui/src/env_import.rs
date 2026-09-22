use std::collections::BTreeMap;

use crate::config::{Environment, Profile};

/// Parse `.env` content into a key→value map. Ignores blank lines and whole-line
/// `#` comments (lines whose first non-whitespace character is `#`). Inline `#`
/// after a value is intentionally NOT stripped, since `.env` values such as
/// passwords may legitimately contain `#`. Strips surrounding single or double
/// quotes from values.
pub fn parse_env(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim().trim_matches(['"', '\'']).to_string();
        map.insert(key, value);
    }
    map
}

/// The profile name to use when the caller names none.
/// A blank value counts as unset, as it does when resolving the environment
/// itself, so a profile never takes an empty name.
pub fn default_profile_name(content: &str) -> String {
    parse_env(content)
        .get("ENVIRONMENT")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "imported".to_string())
}

/// Build a `Profile` (and an optional environment override) from `.env` content.
pub fn import(content: &str, profile_name: &str) -> (Profile, Option<(String, Environment)>) {
    let map = parse_env(content);
    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    // The binary chooses devnet for its own import path, and the
    // library still makes every caller choose: `Network` has no
    // `Default`. Naming the variant rather than the string keeps this
    // fallback from drifting from the map keys
    // `Config::builtin_environments` produces.
    // A blank value counts as unset, as it does in `examples/shared.rs`.
    // `ENVIRONMENT=` would otherwise name an environment no table holds, and
    // `Config::resolved_environment` fails on it.
    let env_name = map
        .get("ENVIRONMENT")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| cbtc::Network::Devnet.to_string());

    let profile = Profile {
        name: profile_name.to_string(),
        environment: env_name.clone(),
        ledger_host: get("LEDGER_HOST"),
        keycloak_host: get("KEYCLOAK_HOST"),
        keycloak_realm: get("KEYCLOAK_REALM"),
        keycloak_client_id: get("KEYCLOAK_CLIENT_ID"),
        keycloak_username: get("KEYCLOAK_USERNAME"),
        keycloak_password: get("KEYCLOAK_PASSWORD"),
        last_selected_party: None,
    };

    let override_env = if map.contains_key("REGISTRY_URL")
        || map.contains_key("DECENTRALIZED_PARTY_ID")
        || map.contains_key("BITSAFE_API_URL")
    {
        Some((
            env_name,
            Environment {
                registry_url: get("REGISTRY_URL"),
                decentralized_party_id: get("DECENTRALIZED_PARTY_ID"),
                bitsafe_api_url: get("BITSAFE_API_URL"),
            },
        ))
    } else {
        None
    };

    (profile, override_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# comment
ENVIRONMENT=devnet
LEDGER_HOST=https://ledger.example
KEYCLOAK_HOST="https://kc.example"
KEYCLOAK_REALM=myrealm
KEYCLOAK_CLIENT_ID=cli
KEYCLOAK_USERNAME=alice
KEYCLOAK_PASSWORD='sekret'
REGISTRY_URL=https://reg.example
DECENTRALIZED_PARTY_ID=cbtc-network::1220ab
BITSAFE_API_URL=https://api.example
"#;

    #[test]
    fn parse_env_strips_comments_and_quotes() {
        // Act
        let map = parse_env(SAMPLE);
        // Assert
        assert_eq!(map.get("LEDGER_HOST").unwrap(), "https://ledger.example");
        assert_eq!(map.get("KEYCLOAK_HOST").unwrap(), "https://kc.example");
        assert_eq!(map.get("KEYCLOAK_PASSWORD").unwrap(), "sekret");
        assert!(!map.contains_key("# comment"));
    }

    #[test]
    fn import_builds_profile_and_env_override() {
        // Act
        let (profile, env) = import(SAMPLE, "imported");
        // Assert
        assert_eq!(profile.name, "imported");
        assert_eq!(profile.environment, "devnet");
        assert_eq!(profile.ledger_host, "https://ledger.example");
        assert_eq!(profile.keycloak_username, "alice");
        assert_eq!(profile.keycloak_password, "sekret");
        let (env_name, ov) = env.expect("env override");
        assert_eq!(env_name, "devnet");
        assert_eq!(ov.registry_url, "https://reg.example");
        assert_eq!(ov.bitsafe_api_url, "https://api.example");
    }

    /// The shipped `.env.example` comments out all three per-network
    /// variables, so importing a copy of it writes no environment override.
    ///
    /// It matters once a value moves. A user then uncomments the one
    /// variable that moved and re-imports, `get` fills the other two with
    /// `""`, and `resolved_environment` fills those from the built-in.
    #[test]
    fn importing_the_shipped_env_example_writes_no_override() {
        let (profile, override_env) = import(include_str!("../../.env.example"), "imported");
        assert_eq!(profile.environment, "devnet");
        assert!(
            override_env.is_none(),
            "the shipped template must pin no per-network value"
        );
    }

    /// A `.env` with no `ENVIRONMENT` key falls back to devnet.
    ///
    /// This is the only test that reaches the `unwrap_or_else`. `SAMPLE`
    /// and the shipped template both set the key, so neither other test
    /// evaluates it.
    #[test]
    fn an_env_without_the_environment_key_falls_back_to_devnet() {
        let (profile, override_env) = import("LEDGER_HOST=https://ledger.example\n", "imported");
        assert_eq!(profile.environment, "devnet");
        assert!(override_env.is_none());
    }

    /// A blank `ENVIRONMENT` falls back too, rather than naming an
    /// environment no table holds.
    ///
    /// `ENVIRONMENT=` parses to an empty value, and an empty name reaches
    /// `Config::resolved_environment`, which then fails with
    /// `environment "" is not configured`. The examples' helper already treats
    /// a blank value as unset, so this keeps the two paths in agreement.
    /// A blank `ENVIRONMENT` names no profile, so the name falls back too.
    ///
    /// `parse_env` trims, so `ENVIRONMENT=` and `ENVIRONMENT=   ` both reach
    /// here as an empty value. Saving a profile under that name stores an
    /// empty key, and `--set-default` then points at it.
    #[test]
    fn a_blank_environment_does_not_name_the_profile() {
        for content in ["ENVIRONMENT=\n", "ENVIRONMENT=   \n"] {
            assert_eq!(default_profile_name(content), "imported", "for {content:?}");
        }
    }

    /// A named `ENVIRONMENT` names the profile.
    #[test]
    fn the_environment_names_the_profile() {
        assert_eq!(default_profile_name("ENVIRONMENT=mainnet\n"), "mainnet");
        assert_eq!(default_profile_name("LEDGER_HOST=x\n"), "imported");
    }

    #[test]
    fn a_blank_environment_falls_back_to_devnet() {
        for content in ["ENVIRONMENT=\n", "ENVIRONMENT=   \n"] {
            let (profile, _) = import(content, "imported");
            assert_eq!(profile.environment, "devnet", "for {content:?}");
        }
    }
}
