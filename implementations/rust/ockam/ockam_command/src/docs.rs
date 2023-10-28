use crate::terminal::TerminalBackground;
use colorful::Colorful;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use syntect::highlighting::Theme;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::Regex,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

const FOOTER: &str = "
Learn More:

Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
Where <SUBCOMMAND> might be: 'node', 'status', 'enroll', etc.
Learn more about Command: https://command.ockam.io/manual/
Learn more about Ockam: https://docs.ockam.io/reference/command

Feedback:

If you have questions, as you explore, join us on the contributors
discord channel https://discord.gg/bewvnm6zqS
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

pub(crate) fn about(text: &str) -> &'static str {
    render(text)
}

pub(crate) fn before_help(text: &str) -> &'static str {
    let mut processed = String::new();
    if is_markdown() {
        processed.push_str(&enrich_preview_tag(text));
    } else {
        processed.push_str(text);
    }
    render(processed.as_str())
}

pub(crate) fn after_help(text: &str) -> &'static str {
    let mut processed = String::new();
    if is_markdown() {
        processed.push_str("### Examples\n\n");
        processed.push_str(text);
    } else {
        processed.push_str("Examples:\n\n");
        processed.push_str(text);
        processed.push_str(FOOTER);
    }
    render(processed.as_str())
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

/// Use a shell syntax highlighter to render the fenced code blocks in terminals
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

const PREVIEW_TOOLTIP_TEXT: &str = include_str!("./static/preview_tooltip.txt");

/// Enrich the `[Preview]` tag with html
fn enrich_preview_tag(text: &str) -> String {
    // Converts [Preview] to <div class="chip t">Preview<div class="tt">..</div></div>
    let mut tooltip = String::new();
    for line in PREVIEW_TOOLTIP_TEXT.trim_end().lines() {
        tooltip.push_str(&format!("<p>{}</p>", line));
    }
    tooltip = format!("<div class=\"tt\">{tooltip}</div>");
    let preview = "<b>Preview</b>";
    let container = format!("<div class=\"chip t\">{}{}</div>", preview, tooltip);
    text.replace("[Preview]", &container)
}
