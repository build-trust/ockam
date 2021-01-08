use crate::error::{Error, FfiOckamError};
use crate::kex_types::*;
use crate::mutex_storage::FfiObjectMutexStorage;
use crate::vault::{DEFAULT_VAULTS, FILESYSTEM_VAULTS, SECRETS};
use crate::vault_types::{FfiVaultFatPointer, FfiVaultType};
use ockam_kex::{CipherSuite, KeyExchanger};
use ockam_kex_xx::{SymmetricState, XXInitiator, XXResponder, XXVault};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

lazy_static! {
    pub(crate) static ref XX_INITIATOR: FfiObjectMutexStorage<XXInitiator> =
        FfiObjectMutexStorage::default();
    pub(crate) static ref XX_RESPONDER: FfiObjectMutexStorage<XXResponder> =
        FfiObjectMutexStorage::default();
}

fn call<F, R>(context: FfiKexFatPointer, callback: F) -> Result<R, FfiOckamError>
where
    F: FnOnce(&mut dyn KeyExchanger) -> Result<R, FfiOckamError>,
{
    match context.kex_type {
        FfiKexType::XxInitiator => {
            let item = XX_INITIATOR.get_object(context.handle)?;
            let mut item = item.lock().unwrap();

            callback(item.deref_mut())
        }
        FfiKexType::XxResponder => {
            let item = XX_RESPONDER.get_object(context.handle)?;
            let mut item = item.lock().unwrap();

            callback(item.deref_mut())
        }
    }
}

fn cast_vault(vault: FfiVaultFatPointer) -> Result<Arc<Mutex<dyn XXVault>>, FfiOckamError> {
    let vault: Arc<Mutex<dyn XXVault>> = match vault.vault_type {
        FfiVaultType::Software => DEFAULT_VAULTS.get_object(vault.handle)?,
        FfiVaultType::Filesystem => FILESYSTEM_VAULTS.get_object(vault.handle)?,
    };

    Ok(vault)
}

/// Create a new kex initiator and return it
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator(
    context: &mut FfiKexFatPointer,
    vault: FfiVaultFatPointer,
    identity_key: u64,
) -> FfiOckamError {
    let res = || {
        let vault = cast_vault(vault)?;
        let identity_key = SECRETS.get_object(identity_key)?;

        let ss = SymmetricState::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            Some(identity_key.clone()),
        );

        let initiator = XXInitiator::new(ss, true);

        let handle = XX_INITIATOR.insert_object(Arc::new(Mutex::new(initiator)))?;

        Ok(FfiKexFatPointer {
            handle,
            kex_type: FfiKexType::XxInitiator,
        })
    };
    let res = res();
    *context = match res {
        Ok(c) => c,
        Err(err) => return err,
    };

    FfiOckamError::none()
}

/// Create a new kex responder and return it
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder(
    context: &mut FfiKexFatPointer,
    vault: FfiVaultFatPointer,
    identity_key: u64,
) -> FfiOckamError {
    let res = || {
        let vault = cast_vault(vault)?;
        let identity_key = SECRETS.get_object(identity_key)?;

        let ss = SymmetricState::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            Some(identity_key.clone()),
        );

        let responder = XXResponder::new(ss, true);

        let handle = XX_RESPONDER.insert_object(Arc::new(Mutex::new(responder)))?;

        Ok(FfiKexFatPointer {
            handle,
            kex_type: FfiKexType::XxResponder,
        })
    };
    let res = res();
    *context = match res {
        Ok(c) => c,
        Err(err) => return err,
    };

    FfiOckamError::none()
}

#[no_mangle]
pub extern "C" fn ockam_kex_process(
    context: FfiKexFatPointer,
    data: *const u8,
    data_length: u32,
    response: &mut u8,
    response_size: u32,
    response_length: &mut u32,
) -> FfiOckamError {
    check_buffer!(data);
    *response_length = 0;
    let data = unsafe { std::slice::from_raw_parts(data, data_length as usize) };
    match call(context, |k| -> Result<(), FfiOckamError> {
        let r = k.process(data)?;

        if response_size < r.len() as u32 {
            return Err(Error::BufferTooSmall.into());
        }
        *response_length = r.len() as u32;
        unsafe { std::ptr::copy_nonoverlapping(r.as_ptr(), response, r.len()) };
        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

#[no_mangle]
pub extern "C" fn ockam_kex_is_complete(
    context: FfiKexFatPointer,
    is_complete: &mut bool,
) -> FfiOckamError {
    match call(context, |k| -> Result<(), FfiOckamError> {
        *is_complete = k.is_complete();

        Ok(())
    }) {
        Ok(_) => FfiOckamError::none(),
        Err(err) => err.into(),
    }
}

#[no_mangle]
pub extern "C" fn ockam_kex_finalize(
    context: FfiKexFatPointer,
    completed_exchange: &mut FfiCompletedKeyExchange,
) -> FfiOckamError {
    let res = || {
        let exchange = match context.kex_type {
            FfiKexType::XxInitiator => {
                let item = XX_INITIATOR.remove_object_sized(context.handle)?;
                let item = Box::new(item);
                item.finalize()
            }
            FfiKexType::XxResponder => {
                let item = XX_RESPONDER.remove_object_sized(context.handle)?;
                let item = Box::new(item);
                item.finalize()
            }
        }?;

        let mut public_key = [0u8; 65];
        if exchange.remote_static_public_key.as_ref().len() > 65 {
            return Err(FfiOckamError::from(Error::InvalidPublicKey));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                exchange.remote_static_public_key.as_ref().as_ptr(),
                public_key.as_mut_ptr(),
                exchange.remote_static_public_key.as_ref().len(),
            );
        }

        let encrypt_key = SECRETS.insert_object(exchange.encrypt_key)?;
        let decrypt_key = SECRETS.insert_object(exchange.decrypt_key)?;

        Ok(FfiCompletedKeyExchange {
            h: exchange.h,
            encrypt_key,
            decrypt_key,
            remote_static_public_key: public_key,
            remote_static_public_key_len: exchange.remote_static_public_key.as_ref().len(),
        })
    };
    let res = res();

    *completed_exchange = match res {
        Ok(c) => c,
        Err(err) => return err,
    };

    FfiOckamError::none()
}
