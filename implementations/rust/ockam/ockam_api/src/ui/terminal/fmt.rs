pub const PADDING: &str = "     ";
pub const ICON_PADDING: &str = "   ";

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
        format!("{}{}\n{}{}",
        $crate::terminal::ICON_PADDING,
        "─".repeat(85).dim().light_gray(),
        $crate::terminal::PADDING,
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{}{}\n{}{}",
        $crate::terminal::ICON_PADDING,
        "─".repeat(85).dim().light_gray(),
        $crate::terminal::PADDING,
        format!($input, $($args),+))
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
