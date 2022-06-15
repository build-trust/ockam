mod encryptor;
pub(crate) use encryptor::*;
mod decryptor;
pub(crate) use decryptor::*;
mod listener;
pub(crate) use listener::*;
mod messages;
pub(crate) use messages::*;
mod trust_policy;
pub use trust_policy::*;
pub mod access_control;
mod local_info;
pub use local_info::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::access_control::IdentityAccessControlBuilder;
    use crate::{Identity, IdentityTrait};
    use core::sync::atomic::{AtomicU8, Ordering};
    use ockam_core::compat::sync::Arc;
    use ockam_core::{route, Any, Result, Route, Routed, Worker};
    use ockam_node::{Context, WorkerBuilder};
    use ockam_vault::Vault;
    use std::time::Duration;
    use tokio::time::sleep;

    #[ockam_macros::test]
    async fn test_channel(ctx: &mut Context) -> Result<()> {
        let alice_vault = Vault::create();
        let bob_vault = Vault::create();

        let alice = Identity::create(ctx, &alice_vault).await?;
        let bob = Identity::create(ctx, &bob_vault).await?;

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

        let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
        assert_eq!(local_info.their_identity_id(), &alice.identifier().await?);

        let return_route = msg.return_route();
        assert_eq!("Hello, Bob!", msg.body());

        ctx.send(return_route, "Hello, Alice!".to_string()).await?;

        let msg = ctx.receive::<String>().await?.take();

        let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
        assert_eq!(local_info.their_identity_id(), &bob.identifier().await?);

        assert_eq!("Hello, Alice!", msg.body());

        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn test_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let alice = Identity::create(ctx, &vault).await?;
        let bob = Identity::create(ctx, &vault).await?;

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

    #[ockam_macros::test]
    async fn test_double_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let alice = Identity::create(ctx, &vault).await?;
        let bob = Identity::create(ctx, &vault).await?;

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

    #[ockam_macros::test]
    async fn test_many_times_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let alice = Identity::create(ctx, &vault).await?;
        let bob = Identity::create(ctx, &vault).await?;

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

    struct Receiver {
        received_count: Arc<AtomicU8>,
    }

    #[ockam_core::async_trait]
    impl Worker for Receiver {
        type Message = Any;
        type Context = Context;

        async fn handle_message(
            &mut self,
            _context: &mut Self::Context,
            _msg: Routed<Self::Message>,
        ) -> Result<()> {
            self.received_count.fetch_add(1, Ordering::Relaxed);

            Ok(())
        }
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn access_control__known_participant__should_pass_messages(
        ctx: &mut Context,
    ) -> Result<()> {
        let received_count = Arc::new(AtomicU8::new(0));
        let receiver = Receiver {
            received_count: received_count.clone(),
        };

        let vault = Vault::create();

        let alice = Identity::create(ctx, &vault).await?;
        let bob = Identity::create(ctx, &vault).await?;

        let access_control = IdentityAccessControlBuilder::new_with_id(alice.identifier().await?);
        WorkerBuilder::with_access_control(access_control, "receiver", receiver)
            .start(ctx)
            .await?;

        bob.create_secure_channel_listener("listener", TrustEveryonePolicy)
            .await?;

        let alice_channel = alice
            .create_secure_channel("listener", TrustEveryonePolicy)
            .await?;

        ctx.send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
            .await?;

        sleep(Duration::from_secs(1)).await;

        assert_eq!(received_count.load(Ordering::Relaxed), 1);

        ctx.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn access_control__unknown_participant__should_not_pass_messages(
        ctx: &mut Context,
    ) -> Result<()> {
        let received_count = Arc::new(AtomicU8::new(0));
        let receiver = Receiver {
            received_count: received_count.clone(),
        };

        let vault = Vault::create();

        let alice = Identity::create(ctx, &vault).await?;
        let bob = Identity::create(ctx, &vault).await?;

        let access_control = IdentityAccessControlBuilder::new_with_id(bob.identifier().await?);
        WorkerBuilder::with_access_control(access_control, "receiver", receiver)
            .start(ctx)
            .await?;

        bob.create_secure_channel_listener("listener", TrustEveryonePolicy)
            .await?;

        let alice_channel = alice
            .create_secure_channel("listener", TrustEveryonePolicy)
            .await?;

        ctx.send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
            .await?;

        sleep(Duration::from_secs(1)).await;

        assert_eq!(received_count.load(Ordering::Relaxed), 0);

        ctx.stop().await
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn access_control__no_secure_channel__should_not_pass_messages(
        ctx: &mut Context,
    ) -> Result<()> {
        let received_count = Arc::new(AtomicU8::new(0));
        let receiver = Receiver {
            received_count: received_count.clone(),
        };

        let access_control = IdentityAccessControlBuilder::new_with_id(
            "P79b26ba2ea5ad9b54abe5bebbcce7c446beda8c948afc0de293250090e5270b6".try_into()?,
        );
        WorkerBuilder::with_access_control(access_control, "receiver", receiver)
            .start(ctx)
            .await?;

        ctx.send(route!["receiver"], "Hello, Bob!".to_string())
            .await?;

        sleep(Duration::from_secs(1)).await;

        assert_eq!(received_count.load(Ordering::Relaxed), 0);

        ctx.stop().await
    }
}
