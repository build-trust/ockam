use crate::environment::compile_time_vars::{BIN_NAME, BRAND_NAME, SUPPORT_EMAIL};
use crate::Result;
use colorful::Colorful;
use ockam_api::terminal::TextHighlighter;
use ockam_core::env::get_env_with_default;
use once_cell::sync::Lazy;
use syntect::{parsing::Regex, util::LinesWithEndings};

const PREVIEW_TOOLTIP_TEXT: &str = include_str!("./static/preview_tooltip.txt");
const PREVIEW_TAG: &str = include_str!("./static/preview_tag.txt");
const UNSAFE_TOOLTIP_TEXT: &str = include_str!("./static/unsafe_tooltip.txt");
const UNSAFE_TAG: &str = include_str!("./static/unsafe_tag.txt");

static IS_MARKDOWN: Lazy<bool> =
    Lazy::new(|| get_env_with_default("OCKAM_HELP_RENDER_MARKDOWN", false).unwrap_or(false));

static HIDE: Lazy<bool> =
    Lazy::new(|| get_env_with_default("OCKAM_HELP_SHOW_HIDDEN", true).unwrap_or(true));

static HEADER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(Examples:|Learn More:|Feedback:).*$".into()));

static FOOTER: Lazy<String> = Lazy::new(|| {
    if BIN_NAME == "ockam" {
        "
Learn More:

Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
Where <SUBCOMMAND> might be: 'node', 'status', 'enroll', etc.
Learn more about Command: https://command.ockam.io/manual/
Learn more about Ockam: https://docs.ockam.io/reference/command

Feedback:

If you have questions, as you explore, join us on the contributors
discord channel https://discord.ockam.io"
            .to_string()
    } else {
        format!(
            "
Learn More:

Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
Where <SUBCOMMAND> might be: 'node', 'status', 'enroll', etc.

Feedback:

If you have questions, please email us on {SUPPORT_EMAIL}"
        )
    }
});

pub(crate) fn hide() -> bool {
    *HIDE
}

pub(crate) fn about(text: &str) -> &'static str {
    render(text)
}

pub(crate) fn before_help(text: &str) -> &'static str {
    let mut processed = String::new();
    if *IS_MARKDOWN {
        if let Some(s) = enrich_preview_tag(text) {
            processed.push_str(&s);
        }
        if let Some(s) = enrich_unsafe_tag(text) {
            processed.push_str(&s);
        }
    } else {
        processed.push_str(text);
    }
    render(processed.as_str())
}

pub(crate) fn after_help(text: &str) -> &'static str {
    let mut processed = String::new();
    if *IS_MARKDOWN {
        processed.push_str("### Examples\n\n");
        processed.push_str(text);
    } else {
        processed.push_str("Examples:\n\n");
        processed.push_str(text);
        processed.push_str(&FOOTER);
    }
    render(processed.as_str())
}

/// Render the string if the document should be displayed in a terminal
/// Otherwise, if it is a Markdown document just return a static string
fn render(body: &str) -> &'static str {
    let body = process_branding(body);
    if *IS_MARKDOWN {
        Box::leak(body.into_boxed_str())
    } else {
        let syntax_highlighted = process_terminal_docs(body);
        Box::leak(syntax_highlighted.into_boxed_str())
    }
}

fn process_branding(text: &str) -> String {
    let mut text = if BRAND_NAME != "Ockam" {
        text.replace("Ockam", BRAND_NAME)
    } else {
        text.to_string()
    };
    text = if BIN_NAME != "ockam" {
        text.replace("ockam", BIN_NAME)
    } else {
        text
    };
    text
}

/// Use a shell syntax highlighter to render the fenced code blocks in terminals
fn process_terminal_docs(input: String) -> String {
    let mut output: Vec<String> = Vec::new();

    for line in LinesWithEndings::from(&input) {
        // Bold and underline known headers
        if HEADER_RE.is_match(line) {
            output.push(line.bold().underlined().to_string());
        }
        // Underline H4 headers
        else if line.starts_with("#### ") {
            output.push(line.replace("#### ", "").underlined().to_string());
        }
        // Remove H5 headers prefix
        else if line.starts_with("##### ") {
            output.push(line.replace("##### ", "").to_string());
        }
        // No processing
        else {
            output.push(line.to_string());
        }
    }
    output.join("")
}

// WARNING: The syntax highlighting is disabled because it has some issues in specific
// machines. For example, in `t2.micro aws Linux us-west-1`, it fails to set the background
// color to the terminal, which adds `11;rgb:0000/0000/0000` before and after the command.
// For now, the syntax highlighting will not be used.
struct FencedCodeBlockHighlighter<'a> {
    inner: TextHighlighter<'a>,
    in_fenced_block: bool,
}

#[allow(dead_code)]
impl FencedCodeBlockHighlighter<'_> {
    fn new() -> Result<Self> {
        Ok(Self {
            inner: TextHighlighter::new("sh")?,
            in_fenced_block: false,
        })
    }

    fn in_fenced_block(&mut self, line: &str) -> bool {
        if line.contains("```") {
            self.in_fenced_block = !self.in_fenced_block;
        }
        self.in_fenced_block
    }

    fn highlight(&mut self, line: &str) -> Result<String> {
        Ok(self.inner.process(line)?)
    }
}

/// Enrich the `[Preview]` tag with html
fn enrich_preview_tag(text: &str) -> Option<String> {
    if !text.contains(PREVIEW_TAG) {
        return None;
    }

    // Converts [Preview] to <div class="chip t">Preview<div class="tt">..</div></div>
    let mut tooltip = String::new();
    for line in PREVIEW_TOOLTIP_TEXT.trim_end().lines() {
        tooltip.push_str(&format!("<p>{}</p>", line));
    }
    tooltip = format!("<div class=\"tt\">{tooltip}</div>");
    let preview_tag = "<b>Preview</b>";
    let container = format!("<div class=\"chip t\">{}{}</div>\n", preview_tag, tooltip);
    Some(text.replace(PREVIEW_TAG, &container))
}

/// Enrich the `[Unsafe]` tag with html
fn enrich_unsafe_tag(text: &str) -> Option<String> {
    if !text.contains(UNSAFE_TAG) {
        return None;
    }

    // Converts [Unsafe] to <div class="chip t">Unsafe<div class="tt">..</div></div>
    let mut tooltip = String::new();
    for line in UNSAFE_TOOLTIP_TEXT.trim_end().lines() {
        tooltip.push_str(&format!("<p>{}</p>", line));
    }
    tooltip = format!("<div class=\"tt\">{tooltip}</div>");
    let unsafe_tag = "<b>Unsafe</b>";
    let container = format!("<div class=\"chip t\">{}{}</div>\n", unsafe_tag, tooltip);
    Some(text.replace(UNSAFE_TAG, &container))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighting() {
        let mut highlighter = FencedCodeBlockHighlighter::new().unwrap();

        // Start of a fenced block
        assert!(highlighter.in_fenced_block("```sh\n"));

        // Highlight line
        let line = highlighter.highlight("echo \"Hello, world!\"\n").unwrap();
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
