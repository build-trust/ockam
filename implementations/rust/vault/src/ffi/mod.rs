use crate::{error::*, software::DefaultVault, Vault};
use ffi_support::{ByteBuffer, ConcurrentHandleMap, ExternError};

/// A context object to interface with C
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OckamVaultContext {
    handle: VaultHandle,
    vault_id: VaultId,
}

/// Represents a Vault id
pub type VaultId = u32;
/// Represents a Vault handle
pub type VaultHandle = u64;
/// Represents a Vault error code
pub type VaultError = u32;
/// No error or success
pub const ERROR_NONE: u32 = 0;

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
    let handle = DEFAULT_VAULTS.insert_with_output(&mut err, DefaultVault::default);
    *context = OckamVaultContext {
        handle,
        vault_id: DEFAULT_VAULT_ID,
    };
    ERROR_NONE
}

/// Fill a preallocated buffer with random data.
/// Can still cause memory seg fault if `buffer` doesn't have enough space to match
/// `buffer_len`. Unfortunately, there is no way to check for this.
#[no_mangle]
pub extern "C" fn ockam_vault_random_bytes_generate(
    context: OckamVaultContext,
    buffer: *mut u8,
    buffer_len: u32,
) -> VaultError {
    check_buffer!(buffer, buffer_len);

    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result_mut(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let mut data = vec![0u8; buffer_len as usize];
                    vault.random(data.as_mut_slice())?;
                    let byte_buffer = ByteBuffer::from_vec(data);
                    Ok(byte_buffer)
                },
            );
            if err.get_code().is_success() {
                let output = output.into_vec();
                unsafe {
                    std::ptr::copy_nonoverlapping(output.as_ptr(), buffer, buffer_len as usize);
                }
                ERROR_NONE
            } else {
                VaultFailErrorKind::Random.into()
            }
        }
        _ => VaultFailErrorKind::Random.into(),
    }
}

/// Compute the SHA-256 hash on `input` and put the result in `digest`.
/// `digest` must be 32 bytes in length
#[no_mangle]
pub extern "C" fn ockam_vault_sha256(context: OckamVaultContext, input: *const u8, input_length: u32, digest: *mut u8) -> VaultError {
    check_buffer!(input, input_length);
    check_buffer!(digest);

    let input = unsafe { std::slice::from_raw_parts(input, input_length as usize) };

    let mut err = ExternError::success();
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            let output = DEFAULT_VAULTS.call_with_result(
                &mut err,
                context.handle,
                |vault| -> Result<ByteBuffer, VaultFailError> {
                    let digest = vault.sha256(input)?;
                    let byte_buffer = ByteBuffer::from_vec(digest.to_vec());
                    Ok(byte_buffer)
                },
            );
            if err.get_code().is_success() {
                let output = output.into_vec();
                unsafe {
                    std::ptr::copy_nonoverlapping(output.as_ptr(), digest, 32);
                }
                ERROR_NONE
            } else {
                VaultFailErrorKind::Random.into()
            }
        }
        _ => VaultFailErrorKind::Random.into(),
    }
}

/// Deinitialize an Ockam vault
#[no_mangle]
pub extern "C" fn ockam_vault_deinit(context: OckamVaultContext) -> VaultError {
    let mut result: VaultError = ERROR_NONE;
    match context.vault_id {
        DEFAULT_VAULT_ID => {
            if DEFAULT_VAULTS.remove_u64(context.handle).is_err() {
                result = VaultFailErrorKind::InvalidContext.into();
            }
        }
        _ => result = VaultFailErrorKind::InvalidContext.into(),
    };
    result
}
