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
mod error;
mod secure_channel;
mod secure_channel_listener;

pub use error::*;
pub use secure_channel::*;
pub use secure_channel_listener::*;

#[cfg(test)]
mod tests {
    use crate::SecureChannel;
    use ockam_core::Route;
    use ockam_vault::SoftwareVault;
    use ockam_vault_sync_core::Vault;

    #[test]
    fn simplest_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault_address = Vault::create(&ctx, SoftwareVault::default()).await?;
                SecureChannel::create_listener(
                    &ctx,
                    "secure_channel_listener".to_string(),
                    vault_address.clone(),
                )
                .await?;
                let initiator = SecureChannel::create(
                    &mut ctx,
                    Route::new().append("secure_channel_listener"),
                    vault_address,
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
