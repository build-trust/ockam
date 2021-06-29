use crate::{ProfileIdentifier, ProfileTrait, SecureChannelTrait};
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Route, Routed};
use ockam_node::Context;
use ockam_vault_sync_core::VaultSync;

mod secure_channel_worker;
pub(crate) use secure_channel_worker::*;
mod listener;
pub(crate) use listener::*;
mod messages;
pub(crate) use messages::*;
mod trust_policy;
pub use trust_policy::*;
mod local_info;
pub use local_info::*;

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
        SecureChannelWorker::create_initiator(ctx, route, self, trust_policy, vault).await
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

// TODO: rename
pub fn check_message_origin<T: Message>(
    msg: &Routed<T>,
    their_profile_id: &ProfileIdentifier,
) -> Result<()> {
    let local_msg = msg.local_message();
    let local_info = LocalInfo::decode(local_msg.local_info())?;

    assert_eq!(local_info.their_profile_id(), their_profile_id);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Profile, ProfileIdentity};
    use ockam_core::{route, Message};
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
                    &ctx,
                    "bob_listener".into(),
                    bob_trust_policy,
                    &vault,
                )
                .await
                .unwrap();

                let alice_channel = alice
                    .create_secure_channel(&ctx, route!["bob_listener"], alice_trust_policy, &vault)
                    .await
                    .unwrap();

                ctx.send(
                    route![alice_channel, ctx.address()],
                    "Hello, Bob!".to_string(),
                )
                .await
                .unwrap();
                let msg = ctx.receive::<String>().await.unwrap().take();

                let local_info = LocalInfo::decode(msg.local_message().local_info()).unwrap();
                assert_eq!(local_info.their_profile_id(), &alice.identifier().unwrap());

                let return_route = msg.return_route();
                assert_eq!("Hello, Bob!", msg.body());

                ctx.send(return_route, "Hello, Alice!".to_string())
                    .await
                    .unwrap();

                let msg = ctx.receive::<String>().await.unwrap().take();

                let local_info = msg.local_message().local_info();

                let local_info = LocalInfo::decode(local_info).unwrap();
                assert_eq!(local_info.their_profile_id(), &bob.identifier().unwrap());

                assert_eq!("Hello, Alice!", msg.body());

                ctx.stop().await.unwrap();
            })
            .unwrap();
    }
}
