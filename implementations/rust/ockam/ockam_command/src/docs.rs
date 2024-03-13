use colorful::Colorful;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use std::time::Duration;
use syntect::util::as_24_bit_terminal_escaped;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme as SyntectTheme, ThemeSet},
    parsing::Regex,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const FOOTER: &str = "
Learn More:

Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
Where <SUBCOMMAND> might be: 'node', 'status', 'enroll', etc.
Learn more about Command: https://command.ockam.io/manual/
Learn more about Ockam: https://docs.ockam.io/reference/command

Feedback:

If you have questions, as you explore, join us on the contributors
discord channel https://discord.ockam.io
";

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static HEADER_RE: Lazy<Regex> = Lazy::new(|| Regex::new("^[A-Za-z][A-Za-z0-9 ]+:$".into()));
static THEME: Lazy<SyntectTheme> = Lazy::new(|| {
    let mut theme_set = ThemeSet::load_defaults();
    let default_theme = theme_set.themes.remove("base16-ocean.dark").unwrap_or(
        theme_set.themes.remove("base16-ocean.light").unwrap_or(
            theme_set
                .themes
                .pop_first()
                .map(|(_, theme)| theme)
                .unwrap_or_default(),
        ),
    );
    match termbg::theme(Duration::from_millis(100)) {
        Ok(termbg::Theme::Light) => theme_set.themes.remove("base16-ocean.light"),
        Ok(termbg::Theme::Dark) => theme_set.themes.remove("base16-ocean.dark"),
        Err(_) => None,
    }
    .unwrap_or(default_theme)
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
fn render(body: &str) -> &'static str {
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

    for line in LinesWithEndings::from(&input) {
        // TODO: disabled because the syntax is adding some unexpected newlines
        // if code_highlighter.in_fenced_block(line) {
        if false {
            output.push(code_highlighter.highlight(line));
        }
        // The line is not part of a fenced block, so process normally.
        else {
            // Replace headers with bold and underline text
            if HEADER_RE.is_match(line) {
                output.push(line.to_string().bold().underlined().to_string());
            }
            // Replace subheaders with underlined text
            else if line.starts_with("#### ") {
                output.push(line.replace("#### ", "").underlined().to_string());
            }
            // No processing
            else {
                output.push(line.to_string());
            }
        }
    }
    output.join("")
}

struct FencedCodeBlockHighlighter<'a> {
    inner: HighlightLines<'a>,
    in_fenced_block: bool,
}

#[allow(dead_code)]
impl FencedCodeBlockHighlighter<'_> {
    fn new() -> Self {
        let syntax = SYNTAX_SET.find_syntax_by_extension("sh").unwrap();
        let theme = &THEME;
        Self {
            inner: HighlightLines::new(syntax, theme),
            in_fenced_block: false,
        }
    }

    fn in_fenced_block(&mut self, line: &str) -> bool {
        if line.contains("```sh") {
            self.in_fenced_block = true;
        } else if line.contains("```\n") {
            self.in_fenced_block = false;
        }
        self.in_fenced_block
    }

    fn highlight(&mut self, line: &str) -> String {
        let highlighted: Vec<(Style, &str)> = self
            .inner
            .highlight_line(line, &SYNTAX_SET)
            .unwrap_or_default();
        as_24_bit_terminal_escaped(&highlighted[..], false)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighting() {
        let mut highlighter = FencedCodeBlockHighlighter::new();

        // Start of a fenced block
        assert!(highlighter.in_fenced_block("```sh\n"));

        // Highlight line
        let line = highlighter.highlight("echo \"Hello, world!\"\n");
        assert!(line.contains("\x1b[38;2;150;181;180m")); // color before "echo"
        assert!(line.contains("\x1b[38;2;192;197;206m")); // color after "echo"

        // Close fenced block
        assert!(!highlighter.in_fenced_block("```\n"));
    }

    #[ignore = "The highlighting is disabled"]
    #[test]
    fn test_process_terminal_docs_with_code_blocks() {
        let input = "```sh
        # To enroll a known identity
        $ ockam project-member add identifier

        # To generate an enrollment ticket that can be used to enroll a device
        $ ockam project ticket --attribute component=control
        ```";

        let result = render(input);
        assert!(
            result.contains("\x1b["),
            "The output should contain ANSI escape codes."
        );
        assert!(
            result.contains("\x1b[0m"),
            "The output should reset ANSI coloring at the end."
        );
    }
}
