use crate::terminal::TerminalBackground;
use colorful::Colorful;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use std::io::{Read, Write};
use syntect::highlighting::Theme;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::Regex,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use termcolor::WriteColor;

const FOOTER: &str = "
Learn More:

Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
Learn more at https://docs.ockam.io/reference/command

Feedback:

If you have any questions or feedback, please start a discussion
on Github https://github.com/build-trust/ockam/discussions/new

Environment Variables:

System
- COLORFGBG: a `string` that defines the foreground and background colors of the terminal.
 If it's not set it has no effect in the Ockam CLI.

CLI Behavior
- NO_COLOR: a `boolean` that, if set, the colors will be stripped out from output messages.
 Otherwise, let the terminal decide.
- NO_INPUT: a `boolean` that, if set, the CLI won't ask the user for input.
 Otherwise, let the terminal decide based the terminal features (tty).
- OCKAM_DISABLE_UPGRADE_CHECK: a `boolean` that, if set, the CLI won't check for ockam upgrades.
- OCKAM_HOME: a `string` that sets the home directory. Defaults to `~/.ockam`.
- OCKAM_LOG: a `string` that defines the verbosity of the logs when the `--verbose` argument is not passed.
- OCKAM_LOG_MAX_SIZE_MB: an `integer` that defines the maximum size of a log file in MB.
- OCKAM_LOG_MAX_FILES: an `integer` that defines the maximum number of log files to keep per node.

Devs Usage
- OCKAM_HELP_SHOW_HIDDEN: a `boolean` to control the visibility of hidden commands.
- OCKAM_CONTROLLER_ADDR: a `string` that overrides the default address of the controller.
- OCKAM_CONTROLLER_IDENTITY_ID: a `string` that overrides the default identifier of the controller.

Internal (to enable some special behavior in the logic)
- OCKAM_HELP_RENDER_MARKDOWN: a `boolean` to control the markdown rendering of the commands documentation.
";

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new("^[A-Za-z][A-Za-z0-9 ]+:$".into()));
static THEME: Lazy<Option<Theme>> = Lazy::new(|| {
    let theme_name = match TerminalBackground::detect_background_color() {
        TerminalBackground::Light => "base16-ocean.light",
        TerminalBackground::Dark => "base16-ocean.dark",
        TerminalBackground::Unknown => return None,
    };
    let mut theme_set = ThemeSet::load_defaults();
    let theme = theme_set.themes.remove(theme_name).unwrap();
    Some(theme)
});

fn is_markdown() -> bool {
    get_env_with_default("OCKAM_HELP_RENDER_MARKDOWN", false).unwrap_or(false)
}

pub(crate) fn hide() -> bool {
    get_env_with_default("OCKAM_HELP_SHOW_HIDDEN", true).unwrap_or(true)
}

pub(crate) fn about(body: &str) -> &'static str {
    render(body)
}

#[allow(unused)]
pub(crate) fn before_help(body: &str) -> &'static str {
    render(body)
}

pub(crate) fn after_help(body: &str) -> &'static str {
    let mut after_help = String::new();
    if is_markdown() {
        after_help.push_str("### Examples\n\n");
        after_help.push_str(body);
    } else {
        after_help.push_str("Examples:\n\n");
        after_help.push_str(body);
        after_help.push_str(FOOTER);
    }
    render(after_help.as_str())
}

/// Render the string if the document should be displayed in a terminal
/// Otherwise, if it is a Markdown document just return a static string
pub(crate) fn render(body: &str) -> &'static str {
    if is_markdown() {
        Box::leak(body.to_string().into_boxed_str())
    } else {
        let syntax_highlighted = process_terminal_docs(body.to_string());
        Box::leak(syntax_highlighted.into_boxed_str())
    }
}

/// Use a shell syntax highlighter to render the code in terminals
fn process_terminal_docs(input: String) -> String {
    let mut output: Vec<String> = Vec::new();
    let mut code_highlighter = FencedCodeBlockHighlighter::new();
    for line in LinesWithEndings::from(input.as_str()) {
        // Try to process fenced code blocks
        if code_highlighter.process_line(line, &mut output) {
            continue;
        }
        // Replace headers with bold and underline text
        if HEADER_RE.is_match(line) {
            output.push(line.to_string().bold().underlined().to_string());
        }
        // Replace subheaders with underlined text
        else if line.starts_with("#### ") {
            output.push(line.replace("#### ", "").underlined().to_string());
        }
        // Catch all
        else {
            output.push(line.to_string());
        }
    }
    output.join("")
}

struct FencedCodeBlockHighlighter<'a> {
    inner: Option<HighlightLines<'a>>,
    in_fenced_block: bool,
}

impl FencedCodeBlockHighlighter<'_> {
    fn new() -> Self {
        let inner = match &*THEME {
            Some(theme) => {
                let syntax = SYNTAX_SET.find_syntax_by_extension("sh").unwrap();
                Some(HighlightLines::new(syntax, theme))
            }
            None => None,
        };
        Self {
            inner,
            in_fenced_block: false,
        }
    }

    fn process_line(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        if let Some(highlighter) = &mut self.inner {
            if line == "```sh\n" {
                self.in_fenced_block = true;
                return true;
            }

            if !self.in_fenced_block {
                return false;
            }

            if line == "```\n" {
                // Push a reset to clear the coloring.
                output.push("\x1b[0m".to_string());
                self.in_fenced_block = false;
                return true;
            }

            // Highlight the code line
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &SYNTAX_SET)
                .unwrap_or_default();
            output.push(as_24_bit_terminal_escaped(&ranges[..], false));
            true
        } else {
            false
        }
    }
}

#[allow(unused)]
fn to_bold_and_underline(mut b: String, s: &str) -> String {
    let mut buffer = termcolor::Buffer::ansi();
    let mut color = termcolor::ColorSpec::new();
    color.set_bold(true);
    color.set_underline(true);
    let err_msg = "Failed to create styled text";
    buffer.set_color(&color).expect(err_msg);
    buffer.write_all(s.as_bytes()).expect(err_msg);
    buffer.reset().expect(err_msg);
    buffer.as_slice().read_to_string(&mut b).expect(err_msg);
    b
}
