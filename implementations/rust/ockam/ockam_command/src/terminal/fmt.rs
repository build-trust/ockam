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
        "  OK  "
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bg_color($crate::terminal::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        "  OK  "
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bg_color($crate::terminal::OckamColor::FmtOKBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}",
        " INFO "
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bg_color($crate::terminal::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        " INFO "
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bg_color($crate::terminal::OckamColor::FmtINFOBackground.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}",
        " WARN "
            .bg_color($crate::terminal::OckamColor::FmtWARNBackground.color())
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        " WARN "
            .bg_color($crate::terminal::OckamColor::FmtWARNBackground.color())
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bold(),
        format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}",
        " ERRO "
            .bg_color($crate::terminal::OckamColor::FmtERRORBackground.color())
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}",
        " ERRO "
            .bg_color($crate::terminal::OckamColor::FmtERRORBackground.color())
            .color($crate::terminal::OckamColor::FmtTextColor.color())
            .bold(),
        format!($input, $($args),+))
    };
}
