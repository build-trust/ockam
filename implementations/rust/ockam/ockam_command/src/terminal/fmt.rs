#[macro_export]
macro_rules! fmt_log {
    ($input:expr) => {
        format!("{} {}", "      ".bg_rgb(82, 199, 234).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "      ".bg_rgb(82, 199, 234).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_ok {
    ($input:expr) => {
        format!("{} {}", "  OK  ".bg_rgb(79, 218, 184).rgb(36, 42, 49).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "  OK  ".bg_rgb(79, 218, 184).rgb(36, 42, 49).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}", " INFO ".bg_rgb(82, 199, 234).rgb(36, 42, 49).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " INFO ".bg_rgb(82, 199, 234).rgb(36, 42, 49).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}", " WARN ".bg_light_yellow().rgb(36, 42, 49).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " WARN ".bg_light_yellow().rgb(36, 42, 49).bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}", " ERR  ".bg_light_red().rgb(36, 42, 49).bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " ERR  ".bg_light_red().rgb(36, 42, 49).bold(), format!($input, $($args),+))
    };
}
