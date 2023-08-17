use ockam_core::env::get_env_with_default;

pub(crate) fn cli_bin() -> crate::Result<String> {
    Ok(get_env_with_default("OCKAM_COMMAND", "ockam".to_string())?)
}
