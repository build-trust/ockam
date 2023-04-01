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
";

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("^[A-Za-z][A-Za-z0-9 ]+:$".into()));
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
    get_env_with_default("OCKAM_HELP_SHOW_HIDDEN", false).unwrap_or(false)
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
/// Otherwise, if it is a Mardown document just return a static string
pub(crate) fn render(body: &str) -> &'static str {
    if is_markdown() {
        Box::leak(body.to_string().into_boxed_str())
    } else {
        let syntax_highlighted = highlight_syntax(body.to_string());
        Box::leak(syntax_highlighted.into_boxed_str())
    }
}

/// Use a shell syntax highlighter to render the code in terminals
fn highlight_syntax(input: String) -> String {
    let mut highlighted: Vec<String> = Vec::new();
    let mut in_fenced_block = false;

    if let Some(theme) = &*THEME {
        let syntax_reference = SYNTAX_SET.find_syntax_by_extension("sh").unwrap();

        let mut highlighter = HighlightLines::new(syntax_reference, theme);
        for line in LinesWithEndings::from(input.as_str()) {
            if line == "```sh\n" {
                in_fenced_block = true;
                continue;
            }

            if !in_fenced_block {
                if RE.is_match(line) {
                    highlighted.push(line.to_string().bold().underlined().to_string());
                } else {
                    highlighted.push(line.to_string());
                }
                continue;
            }

            if line == "```\n" {
                // Push a reset to clear the coloring.
                highlighted.push("\x1b[0m".to_string());
                in_fenced_block = false;
                continue;
            }

            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &SYNTAX_SET)
                .unwrap_or_default();
            highlighted.push(as_24_bit_terminal_escaped(&ranges[..], false));
        }

        highlighted.join("")
    } else {
        input
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
