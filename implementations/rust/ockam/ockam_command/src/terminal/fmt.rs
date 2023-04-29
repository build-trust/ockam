#[macro_export]
macro_rules! fmt_ok {
    ($input:expr) => {
        format!("{} {}", "✔".light_green(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "✔".light_green(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_info {
    ($input:expr) => {
        format!("{} {}", ">".light_green(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", ">".light_green(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_warn {
    ($input:expr) => {
        format!("{} {}", "!".light_yellow(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "!".light_yellow(), format!($input, $($args),+))
    };
}

#[macro_export]
macro_rules! fmt_err {
    ($input:expr) => {
        format!("{} {}", "×".light_red(), format!($input))
    };
    ($input:expr, $($args:expr),+) => {
        format!("{} {}", "×".light_red(), format!($input, $($args),+))
    };
}
