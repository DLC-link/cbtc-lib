use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Network-wide constants for a Canton environment (devnet/testnet/mainnet).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub registry_url: String,
    pub decentralized_party_id: String,
    pub bitsafe_api_url: String,
}

/// A Canton user on a participant node. Secrets are plaintext; the file is 0600.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub environment: String,
    pub ledger_host: String,
    pub keycloak_host: String,
    pub keycloak_realm: String,
    pub keycloak_client_id: String,
    pub keycloak_username: String,
    pub keycloak_password: String,
    #[serde(default)]
    pub last_selected_party: Option<String>,
}

/// Top-level config persisted as TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub environments: BTreeMap<String, Environment>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Config {
    /// Built-in environment defaults shipped with the binary (from `.env.example`).
    pub fn builtin_environments() -> BTreeMap<String, Environment> {
        let mut m = BTreeMap::new();
        m.insert(
            "devnet".to_string(),
            Environment {
                registry_url: "https://api.utilities.digitalasset-dev.com".to_string(),
                decentralized_party_id:
                    "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff"
                        .to_string(),
                bitsafe_api_url: "https://api.devnet.bitsafe.finance".to_string(),
            },
        );
        m.insert(
            "testnet".to_string(),
            Environment {
                registry_url: "https://api.utilities.digitalasset-staging.com".to_string(),
                decentralized_party_id:
                    "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f"
                        .to_string(),
                bitsafe_api_url: "https://api.testnet.bitsafe.finance".to_string(),
            },
        );
        m.insert(
            "mainnet".to_string(),
            Environment {
                registry_url: "https://api.utilities.digitalasset.com".to_string(),
                decentralized_party_id:
                    "cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262"
                        .to_string(),
                bitsafe_api_url: "https://api.mainnet.bitsafe.finance".to_string(),
            },
        );
        m
    }

    /// The environment for `env_name`: a config override if present, else the
    /// built-in default.
    pub fn resolved_environment(&self, env_name: &str) -> Option<Environment> {
        if let Some(env) = self.environments.get(env_name) {
            return Some(env.clone());
        }
        Self::builtin_environments().get(env_name).cloned()
    }

    /// Load config from `path`.
    ///
    /// # Errors
    /// Returns `AppError::Config` if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| AppError::Config(format!("cannot read {}: {e}", path.display())))?;
        toml::from_str(&text).map_err(|e| AppError::Config(format!("invalid TOML: {e}")))
    }

    /// Save config to `path`, creating parent dirs and enforcing `0600` on unix.
    ///
    /// # Errors
    /// Returns `AppError::Config`/`AppError::Io` on serialization or write failure.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|e| AppError::Config(format!("serialize: {e}")))?;
        std::fs::write(path, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

/// Config file location: `$CBTC_TUI_CONFIG`, else `$XDG_CONFIG_HOME/cbtc-tui/config.toml`,
/// else `~/.config/cbtc-tui/config.toml`.
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("CBTC_TUI_CONFIG") {
        return PathBuf::from(p);
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".config"));
    base.join("cbtc-tui").join("config.toml")
}

/// Directory for the rotating log file: `~/.local/state/cbtc-tui`.
pub fn log_dir() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".local/state"));
    base.join("cbtc-tui")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_environments_present() {
        // Arrange / Act
        let envs = Config::builtin_environments();
        // Assert
        assert!(envs.contains_key("devnet"));
        assert!(envs.contains_key("testnet"));
        assert!(envs.contains_key("mainnet"));
        assert_eq!(
            envs["devnet"].registry_url,
            "https://api.utilities.digitalasset-dev.com"
        );
    }

    #[test]
    fn save_then_load_roundtrips() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = Config {
            default_profile: Some("p1".to_string()),
            environments: Default::default(),
            profiles: vec![Profile {
                name: "p1".to_string(),
                environment: "devnet".to_string(),
                ledger_host: "https://ledger.example".to_string(),
                keycloak_host: "https://kc.example".to_string(),
                keycloak_realm: "realm".to_string(),
                keycloak_client_id: "client".to_string(),
                keycloak_username: "alice".to_string(),
                keycloak_password: "secret".to_string(),
                last_selected_party: None,
            }],
        };
        // Act
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        // Assert
        assert_eq!(loaded.default_profile.as_deref(), Some("p1"));
        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].keycloak_username, "alice");
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = Config::default();
        // Act
        cfg.save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        // Assert
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn resolved_environment_prefers_override() {
        // Arrange
        let mut cfg = Config::default();
        cfg.environments.insert(
            "devnet".to_string(),
            Environment {
                registry_url: "https://override".to_string(),
                decentralized_party_id: "dp::1220".to_string(),
                bitsafe_api_url: "https://api".to_string(),
            },
        );
        // Act
        let env = cfg.resolved_environment("devnet").unwrap();
        // Assert
        assert_eq!(env.registry_url, "https://override");
    }
}
