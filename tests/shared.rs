//! Tests for `examples/shared.rs`.
//!
//! `cargo test` compiles every example but runs no test inside one, so a
//! `#[cfg(test)] mod tests` written there would never run. Pulling the file
//! in by path gives it a real test target.

#[path = "../examples/shared.rs"]
mod shared;

use cbtc::Network;
use shared::{
    cross_network_warning, resolve_bitsafe_api_url, resolve_party_id, resolve_registry_url,
};

/// Run `f` and return the message it panicked with, with the panic hook
/// silenced so an expected panic prints no backtrace.
fn panic_message(f: fn() -> String) -> String {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(f);
    std::panic::set_hook(previous);
    let payload = result.expect_err("this call must panic");
    payload
        .downcast_ref::<String>()
        .cloned()
        .expect("every panic in shared.rs is formatted, so the payload is a String")
}

/// Unset all four variables the helper reads.
///
/// # Safety
/// The caller's: one test, one thread, in sequence.
unsafe fn clear() {
    for variable in [
        "ENVIRONMENT",
        "DECENTRALIZED_PARTY_ID",
        "REGISTRY_URL",
        "BITSAFE_API_URL",
    ] {
        unsafe { std::env::remove_var(variable) };
    }
}

/// Every case in one test function.
///
/// Environment variables are process-global and the harness runs tests
/// on threads in parallel, so separate test functions would race on
/// them. Edition 2024 also makes `std::env::set_var` unsafe.
///
/// The `Network` methods appear on the right-hand side of several
/// assertions below. That is correct here: this test covers the
/// resolution rule, and `src/network.rs` pins the values themselves.
#[test]
fn resolution_prefers_the_variable_then_the_network() {
    // SAFETY: one test, one thread, every call below in sequence.
    unsafe {
        // An explicit variable wins over the network.
        clear();
        std::env::set_var("ENVIRONMENT", "mainnet");
        std::env::set_var("DECENTRALIZED_PARTY_ID", "explicit::1220ab");
        assert_eq!(resolve_party_id(), "explicit::1220ab");

        // ENVIRONMENT supplies a value whose own variable is unset.
        assert_eq!(
            resolve_registry_url(),
            cbtc::Network::Mainnet.registry_url()
        );
        assert_eq!(
            resolve_bitsafe_api_url(),
            cbtc::Network::Mainnet.bitsafe_api_url()
        );

        // A blank variable counts as unset.
        std::env::set_var("DECENTRALIZED_PARTY_ID", "   ");
        assert_eq!(
            resolve_party_id(),
            cbtc::Network::Mainnet.decentralized_party_id()
        );

        // Every network resolves, not just the one above.
        for network in cbtc::Network::ALL {
            clear();
            std::env::set_var("ENVIRONMENT", network.to_string());
            assert_eq!(resolve_party_id(), network.decentralized_party_id());
            assert_eq!(resolve_registry_url(), network.registry_url());
            assert_eq!(resolve_bitsafe_api_url(), network.bitsafe_api_url());
        }

        // One variable set, ENVIRONMENT unset: the other two panic.
        clear();
        std::env::set_var("DECENTRALIZED_PARTY_ID", "explicit::1220ab");
        assert_eq!(resolve_party_id(), "explicit::1220ab");
        let message = panic_message(resolve_registry_url);
        assert!(message.contains("REGISTRY_URL"), "{message}");
        let message = panic_message(resolve_bitsafe_api_url);
        assert!(message.contains("BITSAFE_API_URL"), "{message}");

        // An unknown ENVIRONMENT panics and names the three valid values.
        clear();
        std::env::set_var("ENVIRONMENT", "Devnet");
        let message = panic_message(resolve_party_id);
        assert!(message.contains("devnet"), "{message}");
        assert!(message.contains("testnet"), "{message}");
        assert!(message.contains("mainnet"), "{message}");

        // A blank ENVIRONMENT counts as unset too.
        clear();
        std::env::set_var("ENVIRONMENT", "  ");
        let message = panic_message(resolve_party_id);
        assert!(message.contains("DECENTRALIZED_PARTY_ID"), "{message}");
        assert!(message.contains("ENVIRONMENT"), "{message}");

        // Nothing set at all: the message says what to set.
        clear();
        let message = panic_message(resolve_party_id);
        assert!(message.contains("DECENTRALIZED_PARTY_ID"), "{message}");
        assert!(message.contains("ENVIRONMENT"), "{message}");

        // A padded value resolves trimmed.
        clear();
        std::env::set_var("DECENTRALIZED_PARTY_ID", "  explicit::1220ab  ");
        assert_eq!(resolve_party_id(), "explicit::1220ab");

        // A mixed configuration names two networks at once.

        std::env::remove_var("ENVIRONMENT");

        // With no ENVIRONMENT there is nothing to disagree with.
        assert_eq!(
            cross_network_warning(
                "DECENTRALIZED_PARTY_ID",
                Network::Mainnet.decentralized_party_id()
            ),
            None
        );

        std::env::set_var("ENVIRONMENT", "devnet");

        // The override names the network ENVIRONMENT already names.
        assert_eq!(
            cross_network_warning("REGISTRY_URL", Network::Devnet.registry_url()),
            None
        );

        // A custom value names no network, so it draws no warning.
        assert_eq!(
            cross_network_warning("REGISTRY_URL", "https://registry.internal"),
            None
        );

        // A stale party ID left behind after switching ENVIRONMENT.
        let warning = cross_network_warning(
            "DECENTRALIZED_PARTY_ID",
            Network::Mainnet.decentralized_party_id(),
        )
        .expect("a mainnet party ID under ENVIRONMENT=devnet must warn");
        assert!(warning.contains("DECENTRALIZED_PARTY_ID"), "{warning}");
        assert!(warning.contains("mainnet"), "{warning}");
        assert!(warning.contains("devnet"), "{warning}");

        let warning = cross_network_warning("BITSAFE_API_URL", Network::Testnet.bitsafe_api_url())
            .expect("a testnet API URL under ENVIRONMENT=devnet must warn");
        assert!(warning.contains("testnet"), "{warning}");

        std::env::remove_var("ENVIRONMENT");

        clear();
    }
}
