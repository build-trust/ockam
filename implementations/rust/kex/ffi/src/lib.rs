use ffi_support::{ByteBuffer, ConcurrentHandleMap, ExternError, IntoFfi};
use ockam_vault_software::DefaultVault;
use types::*;

mod types;

fn write_bin_to_buffer(
    bin: &[u8],
    buffer: &mut u8,
    buffer_size: u32,
    buffer_length: &mut u32,
) -> KexError {
    if (buffer_size as usize) < bin.len() {
        1
    } else {
        *buffer_length = bin.len() as u32;
        unsafe { std::ptr::copy_nonoverlapping(bin.as_ptr(), buffer, bin.len()) };
        ERROR_NONE
    }
}

/// Create a new kex initiator and return it
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator(context: &mut u64, vault: u64) -> KexError {
    // TODO: obtain vault from storage using handle
    let mut vault = DefaultVault::default();
    let state = match SymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    // TODO: put initiator into storage and return proper handle
    let initiator = Initiator::new(state);
    *context = 0;

    ERROR_NONE
}

/// Create a new kex responder and return it
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder(context: &mut u64, vault: u64) -> KexError {
    // TODO: obtain vault from storage using handle
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    // TODO: put responder into storage and return proper handle
    let responder = Responder::new(state);
    *context = 0;

    ERROR_NONE
}

/// Initiator encodes message 1
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator_encode_message_1(
    context: u64,
    payload: *const u8,
    payload_length: u32,
    m1: &mut u8,
    m1_size: u32,
    m1_length: &mut u32,
) -> KexError {
    // TODO: obtain initiator from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut initiator = Initiator::new(state);

    let payload = unsafe { std::slice::from_raw_parts(payload, payload_length as usize) };

    let message1 = match initiator.encode_message_1(payload) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    write_bin_to_buffer(&message1, m1, m1_size, m1_length)
}

/// Responder encodes message 2
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder_encode_message_2(
    context: u64,
    payload: *const u8,
    payload_length: u32,
    m2: &mut u8,
    m2_size: u32,
    m2_length: &mut u32,
) -> KexError {
    // TODO: obtain responder from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut responder = Responder::new(state);

    let payload = unsafe { std::slice::from_raw_parts(payload, payload_length as usize) };

    let message2 = match responder.encode_message_2(payload) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    write_bin_to_buffer(&message2, m2, m2_size, m2_length)
}

/// Initiator encodes message 3
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator_encode_message_3(
    context: u64,
    payload: *const u8,
    payload_length: u32,
    m3: &mut u8,
    m3_size: u32,
    m3_length: &mut u32,
) -> KexError {
    // TODO: obtain initiator from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut initiator = Initiator::new(state);

    let payload = unsafe { std::slice::from_raw_parts(payload, payload_length as usize) };

    let message3 = match initiator.encode_message_3(payload) {
        Ok(r) => r,
        Err(_) => return 1,
    };

    write_bin_to_buffer(&message3, m3, m3_size, m3_length)
}

/// Responder decodes message 1
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder_decode_message_1(
    context: u64,
    m1: *const u8,
    m1_length: u32,
) -> KexError {
    // TODO: obtain responder from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut responder = Responder::new(state);

    let message1 = unsafe { std::slice::from_raw_parts(m1, m1_length as usize) };

    match responder.decode_message_1(message1) {
        Ok(_) => ERROR_NONE,
        Err(_) => 1,
    }
}

/// Initiator decodes message 2
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator_decode_message_2(
    context: u64,
    m2: *const u8,
    m2_length: u32,
) -> KexError {
    // TODO: obtain initiator from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut initiator = Initiator::new(state);

    let message2 = unsafe { std::slice::from_raw_parts(m2, m2_length as usize) };

    match initiator.decode_message_2(message2) {
        Ok(_) => ERROR_NONE,
        Err(_) => 1,
    }
}

/// Responder decodes message 3
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder_decode_message_3(
    context: u64,
    m3: *const u8,
    m3_length: u32,
) -> KexError {
    // TODO: obtain responder from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut responder = Responder::new(state);

    let message3 = unsafe { std::slice::from_raw_parts(m3, m3_length as usize) };

    match responder.decode_message_3(message3) {
        Ok(_) => ERROR_NONE,
        Err(_) => 1,
    }
}

/// Finalize initiator
#[no_mangle]
pub extern "C" fn ockam_kex_xx_initiator_finalize(context: u64, kex: &mut u64) -> KexError {
    // TODO: obtain initiator from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut initiator = Initiator::new(state);

    // TODO: Can we use vault from Initiator?
    let mut vault2 = DefaultVault::default();
    match initiator.finalize(&mut vault2) {
        Ok(r) => *kex = 0, // TODO: Set proper context
        Err(_) => return 1,
    };

    ERROR_NONE
}

/// Finalize responder
#[no_mangle]
pub extern "C" fn ockam_kex_xx_responder_finalize(context: u64, kex: &mut u64) -> KexError {
    // TODO: obtain initiator from storage
    let mut vault = DefaultVault::default();
    let state = match XXSymmetricState::prologue(&mut vault) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let mut responder = Responder::new(state);

    // TODO: Can we use vault from Initiator?
    let mut vault2 = DefaultVault::default();
    match responder.finalize(&mut vault2) {
        Ok(r) => *kex = 0, // TODO: Set proper context
        Err(_) => return 1,
    };

    ERROR_NONE
}
