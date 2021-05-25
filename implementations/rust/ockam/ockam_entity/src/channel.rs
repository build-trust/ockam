use crate::{ProfileTrait, SecureChannelTrait};
use async_trait::async_trait;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use ockam_vault_sync_core::VaultSync;

mod responder;
pub(crate) use responder::*;
mod initiator;
pub(crate) use initiator::*;
mod listener;
pub(crate) use listener::*;
mod messages;
pub(crate) use messages::*;
mod trust_policy;
pub use trust_policy::*;

#[async_trait]
impl<P: ProfileTrait + Clone> SecureChannelTrait for P {
    /// Create mutually authenticated secure channel
    async fn create_secure_channel(
        &mut self,
        ctx: &Context,
        route: Route,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<Address> {
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        Initiator::create(ctx, route, self, trust_policy, vault).await
    }

    /// Create mutually authenticated secure channel listener
    async fn create_secure_channel_listener(
        &mut self,
        ctx: &Context,
        address: Address,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<()> {
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        let listener = ProfileChannelListener::new(trust_policy, self.clone(), vault);
        ctx.start_worker(address, listener).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Profile, ProfileIdentity};
    use ockam_vault_sync_core::Vault;

    #[test]
    fn test_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let vault = Vault::create(&ctx).unwrap();

                let mut alice = Profile::create(&ctx, &vault).await.unwrap();
                let mut bob = Profile::create(&ctx, &vault).await.unwrap();

                let alice_trust_policy = IdentifierTrustPolicy::new(bob.identifier().unwrap());
                let bob_trust_policy = IdentifierTrustPolicy::new(alice.identifier().unwrap());

                bob.create_secure_channel_listener(
                    &mut ctx,
                    "bob_listener".into(),
                    bob_trust_policy,
                    &vault,
                )
                .await
                .unwrap();

                let alice_channel = alice
                    .create_secure_channel(
                        &mut ctx,
                        Route::new().append("bob_listener").into(),
                        alice_trust_policy,
                        &vault,
                    )
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
