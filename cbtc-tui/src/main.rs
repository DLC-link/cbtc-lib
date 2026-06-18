//! cbtc-tui — interactive terminal UI over the `cbtc` library.
// Dev-only blanket allow while modules are built incrementally; removed in the
// final wiring task once every item has a consumer.
#![allow(dead_code)]

mod config;
mod env_import;
mod error;
mod session;
mod theme;

fn main() {
    println!("cbtc-tui scaffold");
}
