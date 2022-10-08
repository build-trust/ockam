use crate::terminal::{Terminal, TerminalBackground};
use colorful::Colorful;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::Regex,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

const TEMPLATE_BOTTOM: &str = "
Learn More:
    Use 'ockam <SUBCOMMAND> --help' for more information about a subcommand.
    Learn more at https://docs.ockam.io/get-started#command

Feedback:
    If you have any questions or feedback, please start a discussion
    on Github https://github.com/build-trust/ockam/discussions/new
";

pub(crate) fn template(body: &str) -> &'static str {
    let mut template: String = body.to_string();

    template.push_str(TEMPLATE_BOTTOM);
    let highlighted = highlight_syntax(template);

    Box::leak(highlighted.into_boxed_str())
}

pub fn highlight_syntax(input: String) -> String {
    let theme_name = match Terminal::detect_background_color() {
        TerminalBackground::Light => "base16-ocean.light",
        TerminalBackground::Dark => "base16-ocean.dark",
        TerminalBackground::Unknown => return input,
    };
    let theme_set = ThemeSet::load_defaults();
    let theme = &theme_set.themes[theme_name];
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let syntax = syntax_set.find_syntax_by_extension("sh").unwrap();
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut highlighted: Vec<String> = Vec::new();
    let mut in_fenced_block = false;

    for line in LinesWithEndings::from(input.as_str()) {
        if line == "```sh\n" {
            in_fenced_block = true;
            continue;
        }

        if !in_fenced_block {
            let re = Regex::new("^[A-Za-z][A-Za-z0-9 ]+:$".into());
            if re.is_match(line) {
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
            .highlight_line(line, &syntax_set)
            .unwrap_or_default();
        highlighted.push(as_24_bit_terminal_escaped(&ranges[..], false));
    }

    highlighted.join("")
}

pub(crate) fn hide() -> bool {
    match std::env::var("SHOW_HIDDEN") {
        Ok(v) => !v.eq_ignore_ascii_case("true"),
        Err(_e) => true,
    }
}
