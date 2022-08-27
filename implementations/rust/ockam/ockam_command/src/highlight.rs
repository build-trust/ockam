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

pub(crate) fn shell_scripts(script: &str) -> &'static str {
    let mut highlighted: Vec<String> = Vec::new();

    for line in LinesWithEndings::from(HELP_TEMPLATE_TOP) {
        highlighted.push(line.to_string());
    }

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("sh").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-mocha.dark"]);

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
