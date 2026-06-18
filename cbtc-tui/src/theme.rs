use ratatui::style::Color;

/// Semantic color roles (BitSafe brand). Call sites reference roles, never hex.
// Full brand palette; not every role is exercised by the current screen set.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Accent,
    AccentDecorative,
    Fg,
    FgDim,
    Success,
    Warning,
    Danger,
    Info,
}

/// Resolves roles to colors; truecolor hits exact brand hexes, otherwise falls
/// back to the 16 ANSI named colors so it stays legible over basic terminals.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub truecolor: bool,
}

impl Theme {
    /// Detect truecolor support from `$COLORTERM`.
    pub fn detect() -> Theme {
        let truecolor = std::env::var("COLORTERM")
            .map(|v| {
                let v = v.to_ascii_lowercase();
                v.contains("truecolor") || v.contains("24bit")
            })
            .unwrap_or(false);
        Theme { truecolor }
    }

    /// Resolve a semantic [`Role`] to a concrete `ratatui::style::Color`.
    pub fn color(&self, role: Role) -> Color {
        if self.truecolor {
            match role {
                Role::Accent => Color::Rgb(0xD6, 0x3A, 0x0F),
                Role::AccentDecorative => Color::Rgb(0xFF, 0x66, 0x33),
                Role::Fg => Color::Reset,
                Role::FgDim => Color::Rgb(0x8F, 0x7A, 0x6E),
                Role::Success => Color::Rgb(0x22, 0xC5, 0x5E),
                Role::Warning => Color::Rgb(0xEA, 0xB3, 0x08),
                Role::Danger => Color::Rgb(0xEF, 0x44, 0x44),
                Role::Info => Color::Rgb(0x41, 0x8D, 0xF0),
            }
        } else {
            match role {
                Role::Accent | Role::AccentDecorative => Color::Red,
                Role::Fg => Color::Reset,
                Role::FgDim => Color::DarkGray,
                Role::Success => Color::Green,
                Role::Warning => Color::Yellow,
                Role::Danger => Color::Red,
                Role::Info => Color::Blue,
            }
        }
    }
}

/// Mono glyphs (no emoji, ever).
// Brand glyph palette; not every glyph is referenced by the current screens.
#[allow(dead_code)]
pub mod glyph {
    /// Success check mark.
    pub const CHECK: &str = "✓";
    /// Error cross.
    pub const CROSS: &str = "✗";
    /// Warning triangle.
    pub const WARN: &str = "▲";
    /// Decorative diamond.
    pub const DIAMOND: &str = "◆";
    /// Bitcoin symbol.
    pub const BTC: &str = "₿";
    /// Braille spinner frames for loading state.
    pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn truecolor_uses_exact_brand_rgb() {
        // Arrange
        let theme = Theme { truecolor: true };
        // Act / Assert
        assert_eq!(theme.color(Role::Accent), Color::Rgb(0xD6, 0x3A, 0x0F));
        assert_eq!(theme.color(Role::Success), Color::Rgb(0x22, 0xC5, 0x5E));
    }

    #[test]
    fn fallback_uses_named_ansi() {
        // Arrange
        let theme = Theme { truecolor: false };
        // Act / Assert
        assert_eq!(theme.color(Role::Accent), Color::Red);
        assert_eq!(theme.color(Role::Success), Color::Green);
        assert_eq!(theme.color(Role::Fg), Color::Reset);
    }

    #[test]
    fn spinner_has_frames() {
        assert!(!glyph::SPINNER.is_empty());
    }
}
