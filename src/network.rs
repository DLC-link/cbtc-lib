//! The CBTC token, and the networks it is deployed on.
//!
//! The party IDs and the registry URLs are re-exports, so this file
//! writes no literal for them. The Bitsafe API URLs have no upstream
//! home and must not get one: `canton-lib` is a generic Canton library,
//! and `api.mainnet.bitsafe.finance` is our own service.

use std::fmt;
use std::str::FromStr;

/// The CBTC instrument id.
///
/// Naming the ticker is not defaulting it. Every `cbtc` operation still
/// takes its instrument from the caller, because Bitsafe plans to
/// support instruments other than CBTC.
pub const CBTC_TICKER: &str = "CBTC";

/// A CBTC deployment. Carries the per-network values every call needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    Devnet,
    Testnet,
    Mainnet,
}

impl Network {
    /// Every network, in the order a user interface should list them.
    pub const ALL: [Network; 3] = [Self::Devnet, Self::Testnet, Self::Mainnet];

    /// The registrar that administers the CBTC instrument on this
    /// network.
    ///
    /// This value becomes `InstrumentId.admin`, which every holding and
    /// offer filter compares exactly. A wrong one raises no ledger
    /// error: the caller reads a zero balance and no offers.
    pub const fn decentralized_party_id(self) -> &'static str {
        match self {
            Self::Devnet => common::consts::DEVNET_DECENTRALIZED_PARTY_ID,
            Self::Testnet => common::consts::TESTNET_DECENTRALIZED_PARTY_ID,
            Self::Mainnet => common::consts::MAINNET_DECENTRALIZED_PARTY_ID,
        }
    }

    /// Digital Asset's utility registry for this network.
    pub const fn registry_url(self) -> &'static str {
        match self {
            Self::Devnet => registry::consts::DEVNET_REGISTRY_URL,
            Self::Testnet => registry::consts::TESTNET_REGISTRY_URL,
            Self::Mainnet => registry::consts::MAINNET_REGISTRY_URL,
        }
    }

    /// Bitsafe's API for this network. Mint and redeem need it.
    pub const fn bitsafe_api_url(self) -> &'static str {
        match self {
            Self::Devnet => "https://api.devnet.bitsafe.finance",
            Self::Testnet => "https://api.testnet.bitsafe.finance",
            Self::Mainnet => "https://api.mainnet.bitsafe.finance",
        }
    }

    /// The name [`Display`] writes and [`FromStr`] accepts.
    ///
    /// `cbtc-tui` writes these three strings as `[environments.*]` table
    /// names in `config.toml`, and stores one of them in each profile.
    /// Changing one breaks every saved config file.
    const fn name(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// [`Network::from_str`] was given a name that is not one of the three.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNetworkError {
    /// The name that was rejected.
    pub name: String,
}

impl fmt::Display for ParseNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown network {:?}: expected devnet, testnet or mainnet",
            self.name
        )
    }
}

impl std::error::Error for ParseNetworkError {}

impl FromStr for Network {
    type Err = ParseNetworkError;

    /// Exact, lowercase match, following `cbtc-tui`'s own exact `get` at
    /// `config.rs:95`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|network| network.name() == s)
            .ok_or_else(|| ParseNetworkError {
                name: s.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Testing item 1. Nine assertions, each against a literal spelled
    /// out here.
    ///
    /// Six of the nine values are re-exports from `canton-lib`.
    /// Comparing a method against the constant it returns —
    /// `Network::Devnet.registry_url() ==
    /// registry::consts::DEVNET_REGISTRY_URL` — calls the thing under
    /// test and cannot fail. Every literal below is written out instead.
    #[test]
    fn each_network_returns_its_own_three_values() {
        assert_eq!(
            Network::Devnet.decentralized_party_id(),
            "cbtc-network::12202a83c6f4082217c175e29bc53da5f2703ba2675778ab99217a5a881a949203ff"
        );
        assert_eq!(
            Network::Testnet.decentralized_party_id(),
            "cbtc-network::12201b1741b63e2494e4214cf0bedc3d5a224da53b3bf4d76dba468f8e97eb15508f"
        );
        assert_eq!(
            Network::Mainnet.decentralized_party_id(),
            "cbtc-network::12205af3b949a04776fc48cdcc05a060f6bda2e470632935f375d1049a8546a3b262"
        );

        assert_eq!(
            Network::Devnet.registry_url(),
            "https://api.utilities.digitalasset-dev.com"
        );
        assert_eq!(
            Network::Testnet.registry_url(),
            "https://api.utilities.digitalasset-staging.com"
        );
        assert_eq!(
            Network::Mainnet.registry_url(),
            "https://api.utilities.digitalasset.com"
        );

        assert_eq!(
            Network::Devnet.bitsafe_api_url(),
            "https://api.devnet.bitsafe.finance"
        );
        assert_eq!(
            Network::Testnet.bitsafe_api_url(),
            "https://api.testnet.bitsafe.finance"
        );
        assert_eq!(
            Network::Mainnet.bitsafe_api_url(),
            "https://api.mainnet.bitsafe.finance"
        );

        assert_eq!(CBTC_TICKER, "CBTC");
    }

    /// Testing item 2. `Display` and `FromStr` are inverses over all
    /// three variants, in both directions.
    #[test]
    fn display_and_from_str_round_trip() {
        for network in Network::ALL {
            let name = network.to_string();
            assert_eq!(
                name.parse::<Network>().expect("Display's output must parse"),
                network
            );
        }
        assert_eq!("devnet".parse::<Network>().unwrap(), Network::Devnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!(Network::Devnet.to_string(), "devnet");
        assert_eq!(Network::Testnet.to_string(), "testnet");
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
    }

    /// Testing item 3. `"Devnet"` pins the case-sensitivity decision, so
    /// a later reader cannot relax it by accident: these three names are
    /// `config.toml` table names.
    #[test]
    fn from_str_rejects_a_name_that_is_not_one_of_the_three() {
        for name in ["", "Devnet", "DEVNET", "local", "dev", "devnet "] {
            let error = name
                .parse::<Network>()
                .expect_err("only the three lowercase names are valid");
            let message = error.to_string();
            assert!(message.contains("devnet"), "{message}");
            assert!(message.contains("testnet"), "{message}");
            assert!(message.contains("mainnet"), "{message}");
        }
    }

    /// Testing item 4. `ALL` holds three distinct variants.
    ///
    /// The compiler, not this test, catches a variant added to
    /// `Network`. All four methods match on `self` exhaustively, so a
    /// fourth variant stops the crate compiling.
    ///
    /// What nothing catches is a variant added, all four methods
    /// updated, and `ALL` left alone. `ALL` is typed `[Network; 3]`, so
    /// no assertion here can see a variant outside it.
    ///
    /// So this test guards the two things it can. A duplicated entry
    /// would shrink `cbtc-tui`'s environment table, because
    /// `builtin_environments` collects into a `BTreeMap` and a repeated
    /// key overwrites.
    #[test]
    fn all_holds_every_variant_once() {
        assert_eq!(Network::ALL.len(), 3);
        let mut seen = std::collections::HashSet::new();
        for network in Network::ALL {
            assert!(seen.insert(network), "{network} appears twice in ALL");
        }
    }

    /// Every per-network value, for the two file checks below.
    fn nine_values() -> Vec<&'static str> {
        Network::ALL
            .iter()
            .flat_map(|n| {
                [
                    n.decentralized_party_id(),
                    n.registry_url(),
                    n.bitsafe_api_url(),
                ]
            })
            .collect()
    }

    /// Testing item 5. `README.md` documents each of the nine values.
    ///
    /// One direction only: each value must appear somewhere in the file.
    /// That catches a value edited in one place and not the other. It
    /// does not prove the value sits in the right per-network block, and
    /// it does not prove either copy matches the ledger.
    ///
    /// It does not assert `CBTC_TICKER`. `README.md` holds the string
    /// `CBTC` 58 times as prose, so that assertion could never fail.
    #[test]
    fn the_readme_documents_every_per_network_value() {
        let readme = include_str!("../README.md");
        for value in nine_values() {
            assert!(
                readme.contains(value),
                "README.md does not document {value}"
            );
        }
    }

    /// Testing item 8, the inverse of item 5: `.env.example` documents no
    /// per-network value.
    ///
    /// `Network` owns these nine values now. A copy left in this file gets
    /// copied into a `.env`, `dotenvy::dotenv()` loads it into the process
    /// environment, and the precedence rule in `examples/shared.rs` makes it
    /// win. A stale party ID then becomes the instrument admin, and the
    /// caller reads a zero balance rather than an error.
    ///
    /// `include_str!` runs in the compiler, so this check needs no read
    /// access to the file from any tool. It couples the crate's build to the
    /// file's presence, which is acceptable: the file is committed, and the
    /// README tells a user to copy it.
    #[test]
    fn env_example_carries_no_per_network_value() {
        let example = include_str!("../.env.example");
        for value in nine_values() {
            assert!(
                !example.contains(value),
                ".env.example still carries {value}; Network owns it now"
            );
        }
        assert!(
            example.contains("ENVIRONMENT="),
            ".env.example must tell the user which network to pick"
        );
    }
}
