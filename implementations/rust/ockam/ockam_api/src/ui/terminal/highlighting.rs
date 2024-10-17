use crate::Result;
use miette::{miette, IntoDiagnostic};
use once_cell::sync::Lazy;
use std::time::Duration;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub static SYNTAX_SET_NEWLINES: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

pub static THEME: Lazy<Theme> = Lazy::new(|| {
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

pub struct TextHighlighter<'a> {
    pub syntax: &'a SyntaxReference,
    pub theme: &'a Theme,
}

impl<'a> TextHighlighter<'a> {
    pub fn new(syntax: &str) -> Result<Self> {
        let syntax = SYNTAX_SET_NEWLINES
            .find_syntax_by_extension(syntax)
            .ok_or_else(|| miette!("Syntax {syntax} not found"))?;
        Ok(Self {
            syntax,
            theme: &THEME,
        })
    }

    pub fn process(&self, text: &str) -> Result<String> {
        let mut h = HighlightLines::new(self.syntax, self.theme);
        let mut highlighted_text = String::new();
        for line in LinesWithEndings::from(text) {
            let ranges: Vec<(Style, &str)> = h
                .highlight_line(line, &SYNTAX_SET_NEWLINES)
                .into_diagnostic()?;
            highlighted_text.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
        }
        Ok(highlighted_text)
    }
}
