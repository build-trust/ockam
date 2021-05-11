use crate::{ProfileImpl, ProfileVault};
use ockam_core::{Address, Result, Route};
use ockam_node::Context;

mod responder;
pub(crate) use responder::*;
mod initiator;
pub(crate) use initiator::*;
mod listener;
pub(crate) use listener::*;
mod messages;
pub(crate) use messages::*;

impl<V: ProfileVault> ProfileImpl<V> {
    /// Create mutually authenticated secure channel
    pub async fn create_secure_channel<A: Into<Route>>(
        &mut self,
        ctx: &Context,
        route: A,
    ) -> Result<Address> {
        Initiator::create(ctx, route, self).await
    }

    /// Create mutually authenticated secure channel listener
    pub async fn create_secure_channel_listener<A: Into<Address>>(
        &mut self,
        ctx: &Context,
        address: A,
    ) -> Result<()> {
        let clone = self.clone();
        let listener = ProfileChannelListener::new(clone, self.vault());
        ctx.start_worker(address.into(), listener).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Profile;
    use ockam_vault_sync_core::Vault;

    #[test]
    fn test_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create(&ctx).unwrap();

                let mut alice = Profile::create(&ctx, &vault).unwrap();
                let mut bob = Profile::create(&ctx, &vault).unwrap();

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
