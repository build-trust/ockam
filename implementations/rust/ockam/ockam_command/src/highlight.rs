use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

const HELP_TEMPLATE_TOP: &str = "\
{before-help}
{name} {version} {author-with-newline}
{about-with-newline}
{usage-heading}
    {usage}

{all-args}

EXAMPLES
";

const HELP_TEMPLATE_BOTTOM: &str = "
LEARN MORE
    Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
    Learn more at https://docs.ockam.io/get-started#command

FEEDBACK
    If you have any questions or feedback, please start a discussion
    on Github https://github.com/build-trust/ockam/discussions/new
";

pub fn highlighted_shell_script(script: &str) -> &'static str {
    let theme = match TerminalBackground::detect() {
        TerminalBackground::Light => "base16-ocean.dark",
        TerminalBackground::Dark => "base16-ocean.light",
        TerminalBackground::Unknown => return Box::leak(script.to_string().into_boxed_str()),
    };

    let mut highlighted: Vec<String> = Vec::new();
    for line in LinesWithEndings::from(HELP_TEMPLATE_TOP) {
        highlighted.push(line.to_string());
    }

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("sh").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes[theme]);

    for line in LinesWithEndings::from(script) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap_or_default();
        highlighted.push(as_24_bit_terminal_escaped(&ranges[..], false));
    }

    // Push a reset to clear the coloring.
    highlighted.push("\x1b[0m".to_string());

    for line in LinesWithEndings::from(HELP_TEMPLATE_BOTTOM) {
        highlighted.push(line.to_string());
    }

    Box::leak(highlighted.join("").into_boxed_str())
}

enum TerminalBackground {
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
    pub fn detect() -> Self {
        match std::env::var("COLORFGBG") {
            Ok(v) => {
                let parts: Vec<&str> = v.split(';').collect();
                match parts.get(1) {
                    Some(p) => match p.to_string().parse::<i32>() {
                        Ok(i) => {
                            if (0..8).contains(&i) {
                                Self::Dark
                            } else {
                                Self::Light
                            }
                        }
                        Err(_e) => Self::Light,
                    },
                    None => Self::Light,
                }
            }
            Err(_e) => Self::Unknown,
        }
    }
}
