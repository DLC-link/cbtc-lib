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
        .unwrap_or_default()
        .into_iter()
        .collect();

    info!(
        "Fetched {} packages from participant",
        participant_packages.len()
    );

    // Compare expected vs actual
    let mut missing = Vec::new();
    let mut found = Vec::new();

    for (package_id, info) in &manifest.packages {
        if participant_packages.contains(package_id) {
            found.push((package_id.clone(), info.clone()));
        } else {
            missing.push((package_id.clone(), info.clone()));
        }
    }

    let status = if missing.is_empty() {
        DarCheckStatus::Pass
    } else {
        DarCheckStatus::Fail
    };

    // Log summary
    match &status {
        DarCheckStatus::Pass => {
            info!(
                "DAR check passed: all {} expected packages found on participant",
                total_expected
            );
        }
        DarCheckStatus::Fail => {
            error!(
                "DAR check failed: {} of {} packages missing",
                missing.len(),
                total_expected
            );
            for (_, info) in &missing {
                error!("  Missing: {} v{}", info.name, info.version);
            }
        }
    }

    Ok(DarCheckResult {
        status,
        missing,
        found,
        total_expected,
    })
}
