#[macro_export]
macro_rules! fmt_log {
    ($input:expr) => {
        format!("{} {}", "      ", format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "      ", format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_ok {
    ($input:expr) => {
        format!("{} {}",
        "     ✔"
            .color($crate::colors::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     ✔"
            .color($crate::colors::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_para {
    ($input:expr) => {
        format!("{} {}",
        "     │"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     │"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_list {
    ($input:expr) => {
        format!("{} {}",
        "     │"
            .color($crate::colors::OckamColor::FmtLISTBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     │"
            .color($crate::colors::OckamColor::FmtLISTBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_heading {
    ($input:expr) => {
        format!("{}{}\n{} {}",
        "       ",
        "─".repeat(85).dim().dark_gray(),
        "      "
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{}{}\n{} {}",
        "       ",
        "─".repeat(85).dim().dark_gray(),
        "      "
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}",
        "     >"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     >"
            .color($crate::colors::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}",
        "     !"
            .color($crate::colors::OckamColor::FmtWARNBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     !"
            .color($crate::colors::OckamColor::FmtWARNBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}",
        "     ✗"
            .color($crate::colors::OckamColor::FmtERRORBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "     ✗"
            .color($crate::colors::OckamColor::FmtERRORBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}
