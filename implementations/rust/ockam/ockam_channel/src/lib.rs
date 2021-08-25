//! Secure channel types and traits of the Ockam library.
//!
//! This crate contains the secure channel types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

mod error;
mod local_info;
mod secure_channel;
mod secure_channel_listener;
mod secure_channel_worker;
mod traits;

pub use error::*;
pub use local_info::*;
pub use secure_channel::*;
pub use secure_channel_listener::*;
pub use secure_channel_worker::*;
pub use traits::*;

#[cfg(test)]
mod tests {
    use crate::SecureChannel;
    use ockam_core::compat::string::{String, ToString};
    use ockam_core::Route;
    use ockam_key_exchange_core::NewKeyExchanger;
    use ockam_key_exchange_xx::XXNewKeyExchanger;
    use ockam_vault::SoftwareVault;
    use ockam_vault_sync_core::{Vault, VaultSync};

    #[test]
    fn simplest_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create_with_inner(&ctx, SoftwareVault::default())?;
                let vault_sync = VaultSync::create_with_worker(&ctx, &vault).unwrap();
                let new_key_exchanger = XXNewKeyExchanger::new(vault_sync.clone());
                SecureChannel::create_listener_extended(
                    &ctx,
                    "secure_channel_listener".to_string(),
                    new_key_exchanger.clone(),
                    vault_sync.clone(),
                )
                .await?;
                let initiator = SecureChannel::create_extended(
                    &ctx,
                    Route::new().append("secure_channel_listener"),
                    None,
                    new_key_exchanger.initiator()?,
                    vault_sync,
                )
                .await?;

                let test_msg = "Hello, channel".to_string();
                ctx.send(
                    Route::new().append(initiator.address()).append("app"),
                    test_msg.clone(),
                )
                .await?;
                assert_eq!(ctx.receive::<String>().await?, test_msg);
                ctx.stop().await
            })
            .unwrap();
    }
}
