pub(crate) enum TerminalBackground {
    Light,
    Dark,
    Unknown,
}

pub(crate) struct Terminal;

impl Terminal {
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
        match std::env::var("COLORFGBG") {
            Ok(v) => {
                let parts: Vec<&str> = v.split(';').collect();
                match parts.get(1) {
                    Some(p) => match p.to_string().parse::<i32>() {
                        Ok(i) => {
                            if (0..8).contains(&i) {
                                TerminalBackground::Dark
                            } else {
                                TerminalBackground::Light
                            }
                        }
                        Err(_e) => TerminalBackground::Unknown,
                    },
                    None => TerminalBackground::Unknown,
                }
            }
            Err(_e) => TerminalBackground::Unknown,
        }
    }
}
