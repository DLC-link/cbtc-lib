use canton_api_client::apis::configuration::Configuration;
use canton_api_client::apis::default_api;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub cbtc_lib_version: String,
    pub packages: HashMap<String, PackageInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DarCheckStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone)]
pub struct DarCheckResult {
    pub status: DarCheckStatus,
    pub missing: Vec<(String, PackageInfo)>,
    pub found: Vec<(String, PackageInfo)>,
    pub total_expected: usize,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub ledger_host: String,
    pub access_token: String,
    pub manifest_path: String,
}

/// Check whether all required DAR packages are uploaded to the participant.
///
/// Reads the expected packages manifest from `params.manifest_path`,
/// fetches the list of package IDs from the Ledger API, and compares them.
pub async fn check(params: Params) -> Result<DarCheckResult, String> {
    // Read and parse the manifest
    let manifest_content = std::fs::read_to_string(&params.manifest_path)
        .map_err(|e| format!("Failed to read manifest at '{}': {}", params.manifest_path, e))?;

    let manifest: Manifest = serde_json::from_str(&manifest_content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let total_expected = manifest.packages.len();
    info!(
        "Loaded manifest (cbtc-lib v{}): {} expected packages",
        manifest.cbtc_lib_version, total_expected
    );

    // Fetch package IDs from the participant via JSON Ledger API
    let mut config = Configuration::new();
    config.base_path = params.ledger_host.clone();
    config.bearer_access_token = Some(params.access_token.clone());

    let response = default_api::get_v2_packages(&config)
        .await
        .map_err(|e| format!("Failed to fetch packages from participant: {}", e))?;

    let participant_packages: std::collections::HashSet<String> = response
        .package_ids
        .ok_or_else(|| {
            "Failed to fetch packages from participant: response missing 'package_ids' field"
                .to_string()
        })?
        .into_iter()
        .collect();

    info!(
        "Fetched {} packages from participant",
        participant_packages.len()
    );

    let result = compare_packages(&manifest, &participant_packages);

    // Log summary
    match &result.status {
        DarCheckStatus::Pass => {
            info!(
                "DAR check passed: all {} expected packages found on participant",
                result.total_expected
            );
        }
        DarCheckStatus::Fail => {
            error!(
                "DAR check failed: {} of {} packages missing",
                result.missing.len(),
                result.total_expected
            );
            for (_, info) in &result.missing {
                error!("  Missing: {} v{}", info.name, info.version);
            }
        }
    }

    Ok(result)
}

/// Compare expected packages from the manifest against the actual package IDs
/// present on the participant. Returns a structured result with found/missing lists
/// sorted by package name.
pub fn compare_packages(
    manifest: &Manifest,
    participant_packages: &std::collections::HashSet<String>,
) -> DarCheckResult {
    let mut missing = Vec::new();
    let mut found = Vec::new();

    for (package_id, info) in &manifest.packages {
        if participant_packages.contains(package_id) {
            found.push((package_id.clone(), info.clone()));
        } else {
            missing.push((package_id.clone(), info.clone()));
        }
    }

    missing.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    found.sort_by(|a, b| a.1.name.cmp(&b.1.name));

    let status = if missing.is_empty() {
        DarCheckStatus::Pass
    } else {
        DarCheckStatus::Fail
    };

    DarCheckResult {
        status,
        missing,
        found,
        total_expected: manifest.packages.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn manifest_with(packages: Vec<(&str, &str, &str)>) -> Manifest {
        let mut map = HashMap::new();
        for (id, name, version) in packages {
            map.insert(
                id.to_string(),
                PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                },
            );
        }
        Manifest {
            cbtc_lib_version: "0.3.1".to_string(),
            packages: map,
        }
    }

    #[test]
    fn all_packages_present() {
        let manifest = manifest_with(vec![
            ("aaa", "cbtc", "1.1.1"),
            ("bbb", "splice-util", "0.1.4"),
        ]);
        let participant: HashSet<String> =
            ["aaa", "bbb", "ccc"].iter().map(|s| s.to_string()).collect();

        let result = compare_packages(&manifest, &participant);

        assert_eq!(result.status, DarCheckStatus::Pass);
        assert_eq!(result.total_expected, 2);
        assert_eq!(result.found.len(), 2);
        assert!(result.missing.is_empty());
    }

    #[test]
    fn some_packages_missing() {
        let manifest = manifest_with(vec![
            ("aaa", "cbtc", "1.1.1"),
            ("bbb", "splice-util", "0.1.4"),
            ("ccc", "utility-registry-v0", "0.4.0"),
        ]);
        let participant: HashSet<String> = ["aaa"].iter().map(|s| s.to_string()).collect();

        let result = compare_packages(&manifest, &participant);

        assert_eq!(result.status, DarCheckStatus::Fail);
        assert_eq!(result.total_expected, 3);
        assert_eq!(result.found.len(), 1);
        assert_eq!(result.missing.len(), 2);
        // Sorted by name
        assert_eq!(result.missing[0].1.name, "splice-util");
        assert_eq!(result.missing[1].1.name, "utility-registry-v0");
    }

    #[test]
    fn empty_manifest() {
        let manifest = manifest_with(vec![]);
        let participant: HashSet<String> = ["aaa"].iter().map(|s| s.to_string()).collect();

        let result = compare_packages(&manifest, &participant);

        assert_eq!(result.status, DarCheckStatus::Pass);
        assert_eq!(result.total_expected, 0);
        assert!(result.found.is_empty());
        assert!(result.missing.is_empty());
    }

    #[test]
    fn empty_participant() {
        let manifest = manifest_with(vec![("aaa", "cbtc", "1.1.1")]);
        let participant: HashSet<String> = HashSet::new();

        let result = compare_packages(&manifest, &participant);

        assert_eq!(result.status, DarCheckStatus::Fail);
        assert_eq!(result.missing.len(), 1);
        assert_eq!(result.missing[0].1.name, "cbtc");
    }

    #[test]
    fn manifest_deserialization() {
        let json = r#"{
            "cbtc_lib_version": "0.3.1",
            "packages": {
                "abc123": { "name": "cbtc", "version": "1.1.1" }
            }
        }"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.cbtc_lib_version, "0.3.1");
        assert_eq!(manifest.packages.len(), 1);
        assert_eq!(manifest.packages["abc123"].name, "cbtc");
    }

    #[test]
    fn invalid_manifest_json() {
        let json = r#"{ not valid json }"#;
        let result: Result<Manifest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
