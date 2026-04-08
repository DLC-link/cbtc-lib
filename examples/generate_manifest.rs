/// Manifest generator for DAR version checking
///
/// Scans DAR files in cbtc-dars/dars/, extracts the main package ID from each,
/// selects the latest version per package family, and writes expected_packages.json.
///
/// Run with: cargo run --example generate_manifest
///
/// This tool is intended for repo maintainers at release time.
use semver::Version;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Serialize)]
struct Manifest {
    cbtc_lib_version: String,
    packages: HashMap<String, PackageInfo>,
}

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    version: String,
}

/// Parsed info from a single DAR file
struct DarInfo {
    name: String,
    version: Version,
    package_id: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dar_dirs = ["cbtc-dars/dars/dependencies", "cbtc-dars/dars/cbtc"];

    let mut all_dars: Vec<DarInfo> = Vec::new();

    for dir in &dar_dirs {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            eprintln!("Warning: directory not found: {}", dir);
            continue;
        }

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dar") {
                continue;
            }

            match extract_dar_info(&path) {
                Ok(info) => all_dars.push(info),
                Err(e) => eprintln!("Warning: skipping {}: {}", path.display(), e),
            }
        }
    }

    // Group by package name, keep only latest version
    let mut latest_by_name: HashMap<String, DarInfo> = HashMap::new();
    for dar in all_dars {
        let existing = latest_by_name.get(&dar.name);
        if existing.is_none() || existing.unwrap().version < dar.version {
            latest_by_name.insert(dar.name.clone(), dar);
        }
    }

    // Build manifest
    let cbtc_lib_version = env!("CARGO_PKG_VERSION").to_string();
    let mut packages = HashMap::new();
    for dar in latest_by_name.values() {
        packages.insert(
            dar.package_id.clone(),
            PackageInfo {
                name: dar.name.clone(),
                version: dar.version.to_string(),
            },
        );
    }

    let manifest = Manifest {
        cbtc_lib_version,
        packages,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    let output_path = "cbtc-dars/expected_packages.json";
    fs::write(output_path, &json)?;

    println!("Manifest written to {}", output_path);
    println!(
        "  cbtc_lib_version: {}",
        manifest.cbtc_lib_version
    );
    println!("  packages: {}", manifest.packages.len());
    for (pkg_id, info) in &manifest.packages {
        println!("    {} v{} ({}...)", info.name, info.version, &pkg_id[..16]);
    }

    Ok(())
}

/// Extract package name, version, and main package ID from a DAR file.
///
/// DAR files are ZIP archives containing META-INF/MANIFEST.MF.
/// The Name field gives us the DAR name (e.g., "cbtc-1.1.1").
/// The Main-Dalf field path contains the package ID as a 64-char hex hash.
fn extract_dar_info(path: &Path) -> Result<DarInfo, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut manifest_file = archive.by_name("META-INF/MANIFEST.MF")?;

    let mut manifest_content = String::new();
    manifest_file.read_to_string(&mut manifest_content)?;

    // JAR manifests use line wrapping: continuation lines start with a single space.
    // Unwrap them by joining " \n " sequences.
    let unwrapped = manifest_content.replace("\r\n ", "").replace("\n ", "");

    let name = unwrapped
        .lines()
        .find(|l| l.starts_with("Name: "))
        .map(|l| l.strip_prefix("Name: ").unwrap().trim().to_string())
        .ok_or("No Name field in MANIFEST.MF")?;

    let main_dalf = unwrapped
        .lines()
        .find(|l| l.starts_with("Main-Dalf: "))
        .map(|l| l.strip_prefix("Main-Dalf: ").unwrap().trim().to_string())
        .ok_or("No Main-Dalf field in MANIFEST.MF")?;

    // Main-Dalf format: {name}-{version}-{package_id}/{name}-{version}-{package_id}.dalf
    // Extract the package_id (64-char hex) from the directory portion.
    let dir_part = main_dalf
        .split('/')
        .next()
        .ok_or("Invalid Main-Dalf format")?;

    // The package ID is the last 64 characters before the end of the directory name.
    // Directory name format: {dar_name}-{package_id}
    // where dar_name = Name field value (e.g., "cbtc-1.1.1")
    let package_id = dir_part
        .strip_prefix(&format!("{}-", name))
        .ok_or_else(|| format!("Main-Dalf dir '{}' doesn't start with '{}-'", dir_part, name))?
        .to_string();

    if package_id.len() != 64 || !package_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "Expected 64-char hex package ID, got '{}' (len={})",
            package_id,
            package_id.len()
        )
        .into());
    }

    // Parse the version from the Name field.
    // Name format: {package-name}-{semver} (e.g., "utility-commercials-v0-0.2.2")
    // Strategy: try parsing progressively longer suffixes as semver.
    let version = parse_version_from_name(&name)?;
    let pkg_name = name[..name.len() - version.to_string().len() - 1].to_string();

    Ok(DarInfo {
        name: pkg_name,
        version,
        package_id,
    })
}

/// Parse the semver version from a DAR name like "utility-commercials-v0-0.2.2".
/// Tries splitting on '-' from the right and parsing as semver.
fn parse_version_from_name(name: &str) -> Result<Version, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = name.split('-').collect();
    // Try from the rightmost 3 parts joined with '.', then 1 part, etc.
    // Semver requires at least major.minor.patch.
    // DAR names use '-' as separator, so "0.2.2" is a single segment,
    // but some may have "1.0.0" as a single segment too.
    for i in (0..parts.len()).rev() {
        let candidate = parts[i..].join("-");
        // Try parsing the candidate directly (e.g., "0.2.2")
        if let Ok(v) = Version::parse(&candidate) {
            return Ok(v);
        }
    }
    Err(format!("Could not parse semver from DAR name: {}", name).into())
}
