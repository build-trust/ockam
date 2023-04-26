#[macro_export]
macro_rules! fmt_ok {
    ($input:expr) => {
        format!("{} {}", " OK ".bg_light_green().light_gray().bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " OK ".bg_light_green().light_gray().bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}", " INFO ".bg_light_blue().light_gray().bold(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " INFO ".bg_light_blue().light_gray().bold(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}", " WARN ".bg_light_yellow(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " WARN ".bg_light_yellow(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}", " ERROR ".bg_light_red(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", " ERROR ".bg_light_red(), format!($input, $($args),+))
    };
}
