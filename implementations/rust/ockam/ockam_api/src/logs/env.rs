use super::LogFormat;
use ockam_core::env::{get_env, get_env_with_default};

pub fn log_level() -> Option<String> {
    get_env("OCKAM_LOG").unwrap_or_default()
}

pub fn log_max_size_bytes() -> u64 {
    let default = 100;
    get_env_with_default("OCKAM_LOG_MAX_SIZE_MB", default).unwrap_or(default) * 1024 * 1024
}

pub fn log_max_files() -> usize {
    let default: u64 = 60;
    get_env_with_default("OCKAM_LOG_MAX_FILES", default).unwrap_or(default) as usize
}

pub fn log_format() -> LogFormat {
    let default = LogFormat::Default;
    get_env_with_default("OCKAM_LOG_FORMAT", default.clone()).unwrap_or(default)
}
