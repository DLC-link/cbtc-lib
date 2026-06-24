# cbtc-lib — common developer tasks. Run `just` (or `just --list`) to see them.

# Show available recipes.
default:
    @just --list

# Launch the interactive TUI. Extra args pass through, e.g. `just tui --log-level debug`.
tui *args:
    cargo run -p cbtc-tui -- {{args}}

# Launch the TUI built in release mode (smoother rendering).
tui-release *args:
    cargo run -p cbtc-tui --release -- {{args}}

# Import a .env file as a TUI profile, then exit. e.g. `just tui-import .env.mainnet`.
tui-import file *args:
    cargo run -p cbtc-tui -- --import-env {{file}} {{args}}
