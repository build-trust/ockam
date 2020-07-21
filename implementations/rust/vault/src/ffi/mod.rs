use crate::{
    error::*,
    software::DefaultVault,
};
use ffi_support::{ConcurrentHandleMap, ExternError};

/// A context object to interface with C
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OckamVaultContext {
    handle: VaultHandle,
    vault_id: VaultId
}

/// Represents a Vault id
pub type VaultId = u32;
/// Represents a Vault handle
pub type VaultHandle = u64;
/// Represents a Vault error code
pub type VaultError = u32;

lazy_static! {
    static ref DEFAULT_VAULTS: ConcurrentHandleMap<DefaultVault> = ConcurrentHandleMap::new();
}

/// The Default vault id across the FFI boundary
pub const DEFAULT_VAULT_ID: VaultId = 1;

/// Create a new Ockam Default vault and return it
#[no_mangle]
pub extern "C" fn ockam_vault_default_init(context: &mut OckamVaultContext) -> VaultError {
    let mut err = ExternError::success();
    // TODO: handle logging
    let handle = DEFAULT_VAULTS.insert_with_output(&mut err, || {
        DefaultVault::default()
    });
    *context = OckamVaultContext {
        handle,
        vault_id: DEFAULT_VAULT_ID
    };
    0
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(h: OckamVaultContext) -> VaultError {
    let mut result: VaultError = 0;
    match h.vault_id {
        DEFAULT_VAULT_ID => {
            match DEFAULT_VAULTS.remove_u64(h.handle) {
                Err(_) => result = VaultFailErrorKind::InvalidContext.into(),
                Ok(_) => {}
            };
        },
        _ => result = VaultFailErrorKind::InvalidContext.into()
    };
    result
}