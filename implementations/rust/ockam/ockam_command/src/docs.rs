use colorful::Colorful;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use r3bl_ansi_color::{AnsiStyledText, Color, Style as StyleAnsi};
use std::time::Duration;
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
static THEME: Lazy<Option<SyntectTheme>> = Lazy::new(|| {
    let mut theme_set = ThemeSet::load_defaults();
    match termbg::theme(Duration::from_millis(100)) {
        Ok(termbg::Theme::Light) => theme_set.themes.remove("base16-ocean.light"),
        Ok(termbg::Theme::Dark) => theme_set.themes.remove("base16-ocean.dark"),
        Err(_) => None,
    }
});
static DEFAULT_THEME: Lazy<SyntectTheme> = Lazy::new(|| {
    let mut theme_set = ThemeSet::load_defaults();
    theme_set.themes.remove("base16-ocean.dark").unwrap_or(
        theme_set.themes.remove("base16-ocean.light").unwrap_or(
            theme_set
                .themes
                .pop_first()
                .map(|(_, theme)| theme)
                .unwrap_or_default(),
        ),
    )
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
pub fn render(body: &str) -> &'static str {
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
        // Check if the current line is a code block start/end or content.
        let is_code_line = code_highlighter.process_line(line, &mut output);

        // The line is not part of a code block, so process normally.
        if !is_code_line {
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

impl FencedCodeBlockHighlighter<'_> {
    fn new() -> Self {
        let syntax = SYNTAX_SET.find_syntax_by_extension("sh").unwrap();
        let theme = THEME.as_ref().unwrap_or(&DEFAULT_THEME);
        Self {
            inner: HighlightLines::new(syntax, theme),
            in_fenced_block: false,
        }
    }

    fn process_line(&mut self, line: &str, output: &mut Vec<String>) -> bool {
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
        let ranges: Vec<(Style, &str)> = self
            .inner
            .highlight_line(line, &SYNTAX_SET)
            .unwrap_or_default();

        // Convert each syntect range to an ANSI styled string
        Self::convert_syntect_style_to_ansi(output, &ranges);

        true
    }

    /// Convert a vector of syntect ranges to ANSI styled strings
    fn convert_syntect_style_to_ansi(output: &mut Vec<String>, ranges: &Vec<(Style, &str)>) {
        for (style, text) in ranges {
            let ansi_styled_text = AnsiStyledText {
                text,
                style: &[StyleAnsi::Foreground(Color::Rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                ))],
            };

            output.push(ansi_styled_text.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighting() {
        let mut highlighter = FencedCodeBlockHighlighter::new();
        let mut output = Vec::new();

        // Simulate the start of a code block
        assert!(highlighter.process_line("```sh\n", &mut output));

        // Simulate processing a line of code within the code block
        let code_line = "echo \"Hello, world!\"\n";
        let highlighted = highlighter.process_line(code_line, &mut output);

        // We expect this line to be processed (highlighted)
        assert!(highlighted);

        // The output should contain the syntax highlighted version of the code line
        // This is a simplistic check for ANSI escape codes - your actual check might be more complex
        assert!(output.last().unwrap().contains("\x1b["));

        // Simulate the end of a code block
        assert!(highlighter.process_line("```\n", &mut output));

        // Check that the highlighting is reset at the end
        assert!(output.last().unwrap().contains("\x1b[0m"));
    }

    #[test]
    fn test_process_terminal_docs_with_code_blocks() {
        let input = "```sh
        # To enroll a known identity
        $ ockam project ticket --member id_identifier

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
