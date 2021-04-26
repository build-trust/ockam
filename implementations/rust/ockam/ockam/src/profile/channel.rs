use ockam_core::{Address, Result, Route};
use ockam_node::Context;

mod listener;
use crate::{OckamError, Profile, ProfileVault, XXNewKeyExchanger};
pub use listener::*;
use ockam_channel::SecureChannel;

impl<V: ProfileVault> Profile<V> {
    /// Create mutually authenticated secure channel
    pub async fn create_secure_channel<A: Into<Route>>(
        &mut self,
        ctx: &mut Context,
        route: A,
    ) -> Result<Address> {
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.clone());
        let route = route.into();
        let channel =
            SecureChannel::create(ctx, route.clone(), &new_key_exchanger, self.vault.clone())
                .await?;

        let contact = self.to_contact();
        let proof = self.generate_authentication_proof(&channel.auth_hash())?;
        let auth_hash = channel.auth_hash();

        let auth_msg = ChannelAuthMessage::new(auth_hash, contact, proof);

        ctx.send(
            Route::new()
                .append(channel.address())
                .append(route.recipient()), // Assuming last part of the route is listener address
            auth_msg,
        )
        .await?;

        // TODO: Add timeout
        let resp = ctx
            .receive_match(|m: &ChannelAuthMessage| m.auth_hash() == auth_hash)
            .await?
            .take()
            .body();

        let contact = resp.contact();
        if self.contacts().contains_key(contact.identifier()) {
            // TODO: Update profile if needed
        } else {
            self.verify_and_add_contact(contact.clone())?;
        }

        let verified = self.verify_authentication_proof(
            &auth_hash,
            resp.contact().identifier(),
            resp.proof(),
        )?;

        if !verified {
            return Err(OckamError::SecureChannelVerificationFailed.into());
        }

        Ok(channel.address())
    }

    /// Create mutually authenticated secure channel listener
    pub async fn create_secure_channel_listener<A: Into<Address>>(
        &mut self,
        ctx: &mut Context,
        address: A,
    ) -> Result<()> {
        let clone = self.clone();
        let listener = ProfileChannelListener::new(clone, self.vault.clone());
        ctx.start_worker(address.into(), listener).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ProfileBuilder;
    use ockam_vault_sync_core::Vault;

    #[test]
    fn test_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create(&ctx).unwrap();

                let mut alice = ProfileBuilder::create(&ctx, &vault).unwrap();
                let mut bob = ProfileBuilder::create(&ctx, &vault).unwrap();

                bob.create_secure_channel_listener(&mut ctx, "bob_listener")
                    .await
                    .unwrap();

                let alice_channel = alice
                    .create_secure_channel(&mut ctx, Route::new().append("bob_listener"))
                    .await
                    .unwrap();

                ctx.send(
                    Route::new().append(alice_channel).append(ctx.address()),
                    "Hello, Bob!".to_string(),
                )
                .await
                .unwrap();
                let msg = ctx.receive::<String>().await.unwrap().take();
                let return_route = msg.return_route();
                assert_eq!("Hello, Bob!", msg.body());

                ctx.send(return_route, "Hello, Alice!".to_string())
                    .await
                    .unwrap();
                assert_eq!(
                    "Hello, Alice!",
                    ctx.receive::<String>().await.unwrap().take().body()
                );

                ctx.stop().await.unwrap();
            })
            .unwrap();
    }
}
