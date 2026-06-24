use thiserror::Error;

/// Result alias for the TUI's typed errors.
pub type Result<T> = std::result::Result<T, AppError>;

/// Typed errors for cbtc-tui. `cbtc`'s `String` errors are converted into these
/// typed variants at the `ops`/`session` boundary. Note the async task layer
/// (`event`) then flattens them back to `String` for the `app::Event` payloads
/// and `App.error`, since those are display-only on the UI side.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    Config(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("canton error: {0}")]
    Canton(String),

    #[error("operation failed: {0}")]
    Op(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_displays_path() {
        // Arrange
        let err = AppError::Config("bad toml".to_string());
        // Act
        let msg = err.to_string();
        // Assert
        assert_eq!(msg, "config error: bad toml");
    }
}
