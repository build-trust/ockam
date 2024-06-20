/// Left padding for all terminal output
pub const PADDING: &str = "    ";
/// Left padding for all terminal output that starts with an icon
pub const ICON_PADDING: &str = "  ";
/// A two-space indentation for nested terminal output
pub const INDENTATION: &str = "  ";

pub fn get_separator_width() -> usize {
    std::cmp::min(
        r3bl_tuify::get_terminal_width() - PADDING.len(),
        r3bl_tuify::DEFAULT_WIDTH,
    )
}

#[macro_export]
macro_rules! fmt_log {
    ($input:expr) => {
        format!("{}{}",
        $crate::terminal::PADDING,
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{}{}",
        $crate::terminal::PADDING,
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✔"
            .color($crate::colors::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "│"
            .color($crate::colors::OckamColor::FmtLISTBackground.color())
            .bold(),
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("\n{}{}\n{}{}",
        $crate::terminal::PADDING,
        format!($input, $($args),+),
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        ">"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "!"
            .color($crate::colors::OckamColor::FmtWARNBackground.color())
            .bold(),
        format!($input, $($args),+))
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
    ($input:expr, $($args:expr),+) => {
        format!("{}{} {}",
        $crate::terminal::ICON_PADDING,
        "✗"
            .color($crate::colors::OckamColor::FmtERRORBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}
