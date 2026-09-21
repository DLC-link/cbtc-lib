//! Tests for `examples/shared.rs`.
//!
//! `cargo test` compiles every example and runs no test inside one. The
//! Cargo Book states it: "Examples are built by `cargo test` by default
//! to ensure they continue to compile, but they are not *tested* by
//! default." So a `#[cfg(test)] mod tests` written inside
//! `examples/shared.rs` would never run.
//!
//! Pulling the file in by path gives it a real test target.
//! `autoexamples = false` at `Cargo.toml:5` keeps `examples/shared.rs`
//! from becoming an example binary of its own, and no `[[example]]`
//! block names it.

#[path = "../examples/shared.rs"]
mod shared;

use cbtc::Network;
use shared::{
    cross_network_warning, resolve_bitsafe_api_url, resolve_party_id, resolve_registry_url,
};

/// Run `f` and return the message it panicked with.
///
/// The panic hook is silenced for the call, so an expected panic does
/// not print a backtrace into the test output, and restored afterwards.
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
/// resolution rule, and testing item 1 in `src/network.rs` pins the nine
/// values themselves.
#[test]
fn resolution_prefers_the_variable_then_the_network() {
    // SAFETY: one test, one thread. Every set and remove below runs in
    // sequence inside this function, and no other test in this target
    // reads or writes the environment.
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

        // A blank variable counts as unset, so it falls through to the
        // network. A copied template assigns rather than unsets, and an
        // empty party ID becomes an instrument admin that matches no
        // holding.
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

        // One variable set and ENVIRONMENT unset: that function returns,
        // and the other two panic. This is the partial-.env case a
        // single resolve() returning all three would have broken, so it
        // is the case most worth pinning.
        clear();
        std::env::set_var("DECENTRALIZED_PARTY_ID", "explicit::1220ab");
        assert_eq!(resolve_party_id(), "explicit::1220ab");
        let message = panic_message(resolve_registry_url);
        assert!(message.contains("REGISTRY_URL"), "{message}");
        let message = panic_message(resolve_bitsafe_api_url);
        assert!(message.contains("BITSAFE_API_URL"), "{message}");

        // An unknown ENVIRONMENT panics, and the message names the three
        // valid values. "Devnet" pins the case-sensitivity decision.
        clear();
        std::env::set_var("ENVIRONMENT", "Devnet");
        let message = panic_message(resolve_party_id);
        assert!(message.contains("devnet"), "{message}");
        assert!(message.contains("testnet"), "{message}");
        assert!(message.contains("mainnet"), "{message}");

        // A blank ENVIRONMENT counts as unset too, so the message says
        // what to set rather than reporting an empty name as invalid.
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

        // A value that is padded resolves trimmed. dotenvy strips trailing
        // whitespace from a .env line, but a shell `export` does not, and a
        // party ID with a trailing space matches no holding. `cbtc-tui`'s own
        // `parse_env` trims, so the two paths must agree.
        clear();
        std::env::set_var("DECENTRALIZED_PARTY_ID", "  explicit::1220ab  ");
        assert_eq!(resolve_party_id(), "explicit::1220ab");

        // A mixed configuration names two networks at once, and nothing else
        // reports it. The precedence rule works one variable at a time, so an
        // override can hold mainnet's value while ENVIRONMENT says devnet.
        // cross_network_warning fires only when the override holds another
        // *named* network's value, so a custom deployment stays silent.

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

        // A genuinely custom value names no network, so it is not a
        // mistake and draws no warning.
        assert_eq!(
            cross_network_warning("REGISTRY_URL", "https://registry.internal"),
            None
        );

        // The two cases worth catching. A stale party ID left behind
        // after switching ENVIRONMENT, and a half-finished switch that
        // set some variables and not others.
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
