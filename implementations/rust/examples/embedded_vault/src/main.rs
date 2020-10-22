#![feature(alloc_error_handler)]
#![feature(default_alloc_error_handler)]

#![no_std]
#![no_main]

use alloc_cortex_m::CortexMHeap;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

extern crate panic_halt;
extern crate alloc;

use cortex_m::asm;
use cortex_m_rt::entry;
use cortex_m_semihosting::{hprintln};

use core::alloc::Layout;

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    asm::bkpt();

    loop {}
}

use alloc::string::{String};

const HEAP_SIZE: usize = 1024; // in bytes

use ockam_vault::{software::DefaultVault, Vault};
use ockam_vault::types::{SecretKeyAttributes, SecretKeyType, SecretPersistenceType, SecretPurposeType};
use ockam_vault::error_nostd::VaultFailError;

#[entry]
fn main() -> ! {
  unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE) }

  loop {
        let key = vault_example().unwrap();
        assert!(!key.is_empty());
        hprintln!("{:?}", key).unwrap();
  }
}

fn vault_example() -> Result<String,VaultFailError> {
    let mut vault = DefaultVault::default();
    let attributes = SecretKeyAttributes {
        xtype: SecretKeyType::P256,
        persistence: SecretPersistenceType::Ephemeral,
        purpose: SecretPurposeType::KeyAgreement,
    };

    let ctx = vault.secret_generate(attributes)?;
    let exported_key = vault.secret_export(ctx)?;

    Ok(hex::encode(exported_key.as_ref()))
}
