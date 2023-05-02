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
        "  OK  ".color($crate::terminal::OckamColor::DeepBlue.color())
            .bg_color($crate::terminal::OckamColor::SuccessGreen.color())
            .bold(),
        format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "  OK  ".color($crate::terminal::OckamColor::DeepBlue.color()).bg_color($crate::terminal::OckamColor::SuccessGreen.color()).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}", " INFO ".color($crate::terminal::OckamColor::DeepBlue.color()).bg_color($crate::terminal::OckamColor::OckamBlue.color()).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " INFO ".color($crate::terminal::OckamColor::DeepBlue.color()).bg_color($crate::terminal::OckamColor::OckamBlue.color()).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}", " WARN ".bg_light_yellow().color($crate::terminal::OckamColor::DeepBlue.color()).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " WARN ".bg_light_yellow().color($crate::terminal::OckamColor::DeepBlue.color()).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}", " ERR  ".bg_light_red().color($crate::terminal::OckamColor::DeepBlue.color()).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " ERR  ".bg_light_red().color($crate::terminal::OckamColor::DeepBlue.color()), format!($input, $($args),+))
    };
}
