use ockam_core::env::{get_env, FromString};
use ockam_core::errcode::Kind;

pub enum TerminalBackground {
    Light,
    Dark,
    Unknown,
}

impl TerminalBackground {
    /// Detect if terminal background is "light", "dark" or "unknown".
    ///
    /// There are lots of complex heuristics to check this but they all seem
    /// to work in some cases and fail in others. We want to degrade gracefully.
    /// So we rely on the simple tool of whether the COLORFGBG variable is set.
    ///
    /// If it is set, it usually takes the form <foreground-color>:<background-color>
    /// and if <background-color> is in {0,1,2,3,4,5,6,8}, then we assume the terminal
    /// has a dark background.
    ///
    /// Reference: https://stackoverflow.com/a/54652367
    pub fn detect_background_color() -> TerminalBackground {
        let terminal_colors = get_env::<TerminalColors>("COLORFGBG");
        if let Ok(Some(terminal_colors)) = terminal_colors {
            return terminal_colors.terminal_background();
        }

        TerminalBackground::Unknown
    }
}

struct TerminalColors {
    #[allow(dead_code)]
    foreground: Color,
    background: Color,
}

impl TerminalColors {
    pub fn terminal_background(&self) -> TerminalBackground {
        if (0..8).contains(&self.background.0) {
            TerminalBackground::Dark
        } else {
            TerminalBackground::Light
        }
    }
}

impl FromString for TerminalColors {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        let parts: Vec<&str> = s.split(';').collect();
        Ok(TerminalColors {
            foreground: Color::from_string(parts[0])?,
            background: Color::from_string(parts[1])?,
        })
    }
}

struct Color(u8);

impl FromString for Color {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(Color(s.to_string().parse::<u8>().map_err(|_| {
            ockam_core::Error::new(
                ockam_core::errcode::Origin::Core,
                Kind::Internal,
                "u8 parse error",
            )
        })?))
    }
}
