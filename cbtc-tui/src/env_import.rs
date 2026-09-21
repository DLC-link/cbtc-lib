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

/// Build a `Profile` (and an optional environment override) from `.env` content.
pub fn import(content: &str, profile_name: &str) -> (Profile, Option<(String, Environment)>) {
    let map = parse_env(content);
    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    // The binary chooses devnet for its own import path, and the
    // library still makes every caller choose: `Network` has no
    // `Default`. Naming the variant rather than the string keeps this
    // fallback from drifting from the map keys
    // `Config::builtin_environments` produces.
    let env_name = map
        .get("ENVIRONMENT")
        .cloned()
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
    /// Before this change the template assigned all three and an import
    /// wrote one. The effect is the same on the day of the change, because
    /// that override carried the built-in values anyway. It differs once a
    /// value moves: a user then uncomments the one variable that moved and
    /// re-imports, `get` fills the other two with `""`, and
    /// `resolved_environment` fills those from the built-in.
    ///
    /// This test is an addition to the design's eight testing items. The
    /// design records this behaviour change and asks the implementer to
    /// document it; nothing else detects it.
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
    /// **This is the only test that reaches the line this task edits.**
    /// `SAMPLE` sets `ENVIRONMENT=devnet` at `env_import.rs:73`, so
    /// `map.get("ENVIRONMENT")` returns `Some` and
    /// `import_builds_profile_and_env_override` never evaluates the
    /// `unwrap_or_else`. The shipped `.env.example` now sets the key too,
    /// so the test above does not reach it either.
    #[test]
    fn an_env_without_the_environment_key_falls_back_to_devnet() {
        let (profile, override_env) = import("LEDGER_HOST=https://ledger.example\n", "imported");
        assert_eq!(profile.environment, "devnet");
        assert!(override_env.is_none());
    }
}
