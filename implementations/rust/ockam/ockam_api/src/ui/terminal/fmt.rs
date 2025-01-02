/// Left padding for all terminal output
pub const PADDING: &str = "    ";
/// Left padding for all terminal output that starts with an icon
pub const ICON_PADDING: &str = "  ";
/// Left padding for miette errors
pub const MIETTE_PADDING: &str = "  ";
/// A two-space indentation for nested terminal output
pub const INDENTATION: &str = "  ";

pub fn get_separator_width() -> usize {
    // If we can't get the terminal width, use the default width
    let mut terminal_width = r3bl_tuify::get_terminal_width();
    if terminal_width == 0 {
        terminal_width = r3bl_tuify::DEFAULT_WIDTH;
    }
    // Make sure the separator width is at least twice the length of the padding.
    // We want to show a small separator even if the terminal is too narrow.
    let terminal_width = std::cmp::max(terminal_width, 2 * PADDING.len());
    // Limit the separator width to the default width
    std::cmp::min(terminal_width - PADDING.len(), r3bl_tuify::DEFAULT_WIDTH)
}

#[test]
fn can_get_separator_width_when_no_terminal_is_available() {
    let separator_width = get_separator_width();
    // Depending on how the tests are run, the terminal width will be either 0 or the default width,
    // so we can't make any assumptions about the separator width.
    assert!(separator_width > 0);
}

#[macro_export]
macro_rules! fmt_log {
    ($input:expr) => {
        format!("{}{}",
        $crate::terminal::PADDING,
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{}",
        $crate::terminal::PADDING,
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_ok {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✔"
            .color($crate::colors::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✔"
            .color($crate::colors::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_para {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_list {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtLISTBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtLISTBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_heading {
    ($input:expr) => {
        format!("\n{}{}\n{}{}",
        $crate::terminal::PADDING,
        format!("{}", $input),
        $crate::terminal::PADDING,
        "─".repeat($crate::terminal::get_separator_width()).dim().light_gray())
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("\n{}{}\n{}{}",
        $crate::terminal::PADDING,
        format!($input, $($args),*),
        $crate::terminal::PADDING,
        "─".repeat($crate::terminal::get_separator_width()).dim().light_gray())
    };
}

#[macro_export]
macro_rules! fmt_separator {
    () => {
        format!(
            "\n{}{}",
            $crate::terminal::PADDING,
            "─"
                .repeat($crate::terminal::get_separator_width())
                .dim()
                .light_gray()
        )
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        ">"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        ">"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "!"
            .color($crate::colors::OckamColor::FmtWARNBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "!"
            .color($crate::colors::OckamColor::FmtWARNBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✗"
            .color($crate::colors::OckamColor::FmtERRORBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr $(, $args:expr)* $(,)?) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✗"
            .color($crate::colors::OckamColor::FmtERRORBackground.color())
            .bold(),
        format!($input, $($args),*))
    };
}
