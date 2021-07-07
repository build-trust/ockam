use crate::{Identity, ProfileIdentifier};
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
pub trait SecureChannelTrait {
    async fn create_secure_channel_async(
        self,
        ctx: &Context,
        route: Route,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<Address>;

    async fn create_secure_channel_listener_async(
        self,
        ctx: &Context,
        address: Address,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<()>;
}

#[async_trait]
impl<P: Identity + Send> SecureChannelTrait for P {
    /// Create mutually authenticated secure channel
    async fn create_secure_channel_async(
        self,
        ctx: &Context,
        route: Route,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<Address> {
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        SecureChannelWorker::create_initiator(ctx, route, self, trust_policy, vault).await
    }

    /// Create mutually authenticated secure channel listener
    async fn create_secure_channel_listener_async(
        self,
        ctx: &Context,
        address: Address,
        trust_policy: impl TrustPolicy,
        vault: &Address,
    ) -> Result<()> {
        let vault = VaultSync::create_with_worker(ctx, vault)?;
        let listener = ProfileChannelListener::new(trust_policy, self, vault);
        ctx.start_worker(address, listener).await
    }
}

// TODO: rename
pub fn check_message_origin<T: Message>(
    msg: &Routed<T>,
    their_profile_id: &ProfileIdentifier,
) -> Result<bool> {
    let local_msg = msg.local_message();
    let local_info = LocalInfo::decode(local_msg.local_info())?;

    let res = local_info.their_profile_id() == their_profile_id;

    Ok(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Entity, SecureChannels};
    use ockam_core::{route, Message};

    #[test]
    fn disable_test_channel() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                let mut alice = Entity::create(&ctx).unwrap();
                let mut bob = Entity::create(&ctx).unwrap();

                let _alice_trust_policy = IdentifierTrustPolicy::new(bob.identifier().unwrap());
                let _bob_trust_policy = IdentifierTrustPolicy::new(alice.identifier().unwrap());

                bob.create_secure_channel_listener(
                    "bob_listener",
                    // FIXME: bob_trust_policy,
                )
                .unwrap();

                let alice_channel = alice
                    .create_secure_channel(
                        route!["bob_listener"],
                        // FIXME: alice_trust_policy,
                    )
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
