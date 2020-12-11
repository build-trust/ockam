use ockam_vault_software::ockam_vault::error::VaultFailError;
use std::ffi::NulError;

// FIXME: This should be removed after introducing common error

pub(crate) fn map_vault_error(err: VaultFailError) -> ffi_support::ExternError {
    ffi_support::ExternError::new_error(ffi_support::ErrorCode::new(1 as i32), err.to_string())
}

pub(crate) fn map_nul_error(err: NulError) -> ffi_support::ExternError {
    ffi_support::ExternError::new_error(ffi_support::ErrorCode::new(2 as i32), err.to_string())
}
