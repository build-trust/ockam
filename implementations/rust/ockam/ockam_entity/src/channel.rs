use crate::ProfileIdentifier;
use ockam_core::{Decodable, Message, Result, Routed};

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

// TODO: rename
pub fn get_secure_channel_participant_id<T: Message>(msg: &Routed<T>) -> Result<ProfileIdentifier> {
    let local_msg = msg.local_message();
    let local_info = LocalInfo::decode(local_msg.local_info())?;

    let res = local_info.their_profile_id().clone();

    Ok(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Entity, Identity};
    use ockam_core::{route, Route};
    use ockam_node::Context;
    use ockam_vault_sync_core::Vault;

    #[ockam_node_test_attribute::node_test]
    async fn test_channel(ctx: &mut Context) -> Result<()> {
        let alice_vault = Vault::create(&ctx).await.expect("failed to create vault");
        let bob_vault = Vault::create(&ctx).await.expect("failed to create vault");

        let mut alice = Entity::create(&ctx, &alice_vault).await?;
        let mut bob = Entity::create(&ctx, &bob_vault).await?;

        let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier().await?);
        let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier().await?);

        bob.create_secure_channel_listener("bob_listener", bob_trust_policy)
            .await?;

        let alice_channel = alice
            .create_secure_channel(route!["bob_listener"], alice_trust_policy)
            .await?;

        ctx.send(
            route![alice_channel, ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
        let msg = ctx.receive::<String>().await?.take();

        let local_info = LocalInfo::decode(msg.local_message().local_info())?;
        assert_eq!(local_info.their_profile_id(), &alice.identifier().await?);

        let return_route = msg.return_route();
        assert_eq!("Hello, Bob!", msg.body());

        ctx.send(return_route, "Hello, Alice!".to_string()).await?;

        let msg = ctx.receive::<String>().await?.take();

        let local_info = msg.local_message().local_info();

        let local_info = LocalInfo::decode(local_info)?;
        assert_eq!(local_info.their_profile_id(), &bob.identifier().await?);

        assert_eq!("Hello, Alice!", msg.body());

        ctx.stop().await
    }

    #[ockam_node_test_attribute::node_test]
    async fn test_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create(&ctx).await?;

        let mut alice = Entity::create(&ctx, &vault).await?;
        let mut bob = Entity::create(&ctx, &vault).await?;

        let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier().await?);
        let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier().await?);

        bob.create_secure_channel_listener("bob_listener", bob_trust_policy.clone())
            .await?;

        let alice_channel = alice
            .create_secure_channel(route!["bob_listener"], alice_trust_policy.clone())
            .await?;

        bob.create_secure_channel_listener("bob_another_listener", bob_trust_policy)
            .await?;

        let alice_another_channel = alice
            .create_secure_channel(
                route![alice_channel, "bob_another_listener"],
                alice_trust_policy,
            )
            .await?;

        ctx.send(
            route![alice_another_channel, ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
        let msg = ctx.receive::<String>().await?.take();
        let return_route = msg.return_route();
        assert_eq!("Hello, Bob!", msg.body());

        ctx.send(return_route, "Hello, Alice!".to_string()).await?;
        assert_eq!(
            "Hello, Alice!",
            ctx.receive::<String>().await?.take().body()
        );

        ctx.stop().await
    }

    #[ockam_node_test_attribute::node_test]
    async fn test_double_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create(&ctx).await?;

        let mut alice = Entity::create(&ctx, &vault).await?;
        let mut bob = Entity::create(&ctx, &vault).await?;

        let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier().await?);
        let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier().await?);

        bob.create_secure_channel_listener("bob_listener", bob_trust_policy.clone())
            .await?;

        let alice_channel = alice
            .create_secure_channel(route!["bob_listener"], alice_trust_policy.clone())
            .await?;

        bob.create_secure_channel_listener("bob_another_listener", bob_trust_policy.clone())
            .await?;

        let alice_another_channel = alice
            .create_secure_channel(
                route![alice_channel, "bob_another_listener"],
                alice_trust_policy.clone(),
            )
            .await?;

        bob.create_secure_channel_listener("bob_yet_another_listener", bob_trust_policy)
            .await?;

        let alice_yet_another_channel = alice
            .create_secure_channel(
                route![alice_another_channel, "bob_yet_another_listener"],
                alice_trust_policy,
            )
            .await?;

        ctx.send(
            route![alice_yet_another_channel, ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
        let msg = ctx.receive::<String>().await?.take();
        let return_route = msg.return_route();
        assert_eq!("Hello, Bob!", msg.body());

        ctx.send(return_route, "Hello, Alice!".to_string()).await?;
        assert_eq!(
            "Hello, Alice!",
            ctx.receive::<String>().await?.take().body()
        );

        ctx.stop().await
    }

    #[ockam_node_test_attribute::node_test]
    async fn test_many_times_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create(&ctx).await?;

        let mut alice = Entity::create(&ctx, &vault).await?;
        let mut bob = Entity::create(&ctx, &vault).await?;

        let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier().await?);
        let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier().await?);

        let n = rand::random::<u8>() % 5 + 4;
        let mut channels = vec![];
        for i in 0..n {
            bob.create_secure_channel_listener(i.to_string(), bob_trust_policy.clone())
                .await?;
            let channel_route: Route;
            if i > 0 {
                channel_route = route![channels.pop().unwrap(), i.to_string()];
            } else {
                channel_route = route![i.to_string()];
            }
            let alice_channel = alice
                .create_secure_channel(channel_route, alice_trust_policy.clone())
                .await?;
            channels.push(alice_channel);
        }

        ctx.send(
            route![channels.pop().unwrap(), ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
        let msg = ctx.receive::<String>().await?.take();
        let return_route = msg.return_route();
        assert_eq!("Hello, Bob!", msg.body());

        ctx.send(return_route, "Hello, Alice!".to_string()).await?;
        assert_eq!(
            "Hello, Alice!",
            ctx.receive::<String>().await?.take().body()
        );

        ctx.stop().await
    }
}
