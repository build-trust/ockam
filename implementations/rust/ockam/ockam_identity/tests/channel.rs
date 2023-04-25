use core::sync::atomic::{AtomicU8, Ordering};
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::{route, Address, AllowAll, Any, DenyAll, Mailboxes, Result, Routed, Worker};
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::{
    DecryptionResponse, EncryptionRequest, EncryptionResponse, IdentityAccessControlBuilder,
    IdentitySecureChannelLocalInfo, SecureChannelListenerOptions, SecureChannelOptions,
    TrustEveryonePolicy, TrustIdentifierPolicy,
};
use ockam_node::{Context, WorkerBuilder};
use tokio::time::sleep;

#[ockam_macros::test]
async fn test_channel(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier());

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy),
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(
            route![alice_channel, child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(local_info.their_identity_id(), alice.identifier());

    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.body());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(local_info.their_identity_id(), bob.identifier());

    assert_eq!("Hello, Alice!", msg.body());

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_channel_send_multiple_messages_both_directions(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier());

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy),
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    for n in 0..50 {
        let payload = format!("Hello, Bob! {}", n);
        child_ctx
            .send(
                route![alice_channel.clone(), child_ctx.address()],
                payload.clone(),
            )
            .await?;

        let message = child_ctx.receive::<String>().await?;
        assert_eq!(&payload, message.as_body());

        let payload = format!("Hello, Alice! {}", n);
        child_ctx
            .send(message.return_route(), payload.clone())
            .await?;

        let message = child_ctx.receive::<String>().await?;
        assert_eq!(&payload, message.as_body());
    }

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_channel_registry(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let alice_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(&alice_channel)
        .unwrap();

    assert!(alice_channel_data.is_initiator());
    assert_eq!(alice_channel_data.my_id(), alice.identifier());
    assert_eq!(alice_channel_data.their_id(), bob.identifier());

    let mut bob_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "bob",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.send(
        route![alice_channel.clone(), "bob"],
        "Hello, Alice!".to_string(),
    )
    .await?;

    let msg = bob_ctx.receive::<String>().await?;
    let return_route = msg.return_route();

    assert_eq!("Hello, Alice!", msg.body());

    let bob_channel = return_route.next().unwrap().clone();

    let bob_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(&bob_channel)
        .unwrap();

    assert!(!bob_channel_data.is_initiator());
    assert_eq!(bob_channel_data.my_id(), bob.identifier());
    assert_eq!(bob_channel_data.their_id(), alice.identifier());

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_channel_api(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let mut bob_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "bob",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.send(
        route![alice_channel.clone(), "bob"],
        "Hello, Alice!".to_string(),
    )
    .await?;

    let msg = bob_ctx.receive::<String>().await?;
    let return_route = msg.return_route();

    assert_eq!("Hello, Alice!", msg.body());

    let bob_channel = return_route.next().unwrap().clone();

    let alice_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(&alice_channel)
        .unwrap();

    let bob_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(&bob_channel)
        .unwrap();

    let encrypted_alice: EncryptionResponse = ctx
        .send_and_receive(
            route![alice_channel_data.encryptor_api_address().clone()],
            EncryptionRequest(b"Ping".to_vec()),
        )
        .await?;
    let encrypted_alice = match encrypted_alice {
        EncryptionResponse::Ok(p) => p,
        EncryptionResponse::Err(err) => return Err(err),
    };

    let encrypted_bob: EncryptionResponse = ctx
        .send_and_receive(
            route![bob_channel_data.encryptor_api_address().clone()],
            EncryptionRequest(b"Pong".to_vec()),
        )
        .await?;
    let encrypted_bob = match encrypted_bob {
        EncryptionResponse::Ok(p) => p,
        EncryptionResponse::Err(err) => return Err(err),
    };

    let decrypted_alice: DecryptionResponse = ctx
        .send_and_receive(
            route![alice_channel_data.decryptor_api_address().clone()],
            encrypted_bob,
        )
        .await?;
    let decrypted_alice = match decrypted_alice {
        DecryptionResponse::Ok(p) => p,
        DecryptionResponse::Err(err) => return Err(err),
    };

    let decrypted_bob: DecryptionResponse = ctx
        .send_and_receive(
            route![bob_channel_data.decryptor_api_address().clone()],
            encrypted_alice,
        )
        .await?;
    let decrypted_bob = match decrypted_bob {
        DecryptionResponse::Ok(p) => p,
        DecryptionResponse::Err(err) => return Err(err),
    };

    assert_eq!(decrypted_alice, b"Pong");
    assert_eq!(decrypted_bob, b"Ping");

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier());

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone()),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_another_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy),
        )
        .await?;

    let alice_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_channel, "bob_another_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy),
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(
            route![alice_another_channel, child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.body());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!("Hello, Alice!", child_ctx.receive::<String>().await?.body());

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_double_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier());

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone()),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_another_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone()),
        )
        .await?;

    let alice_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_channel, "bob_another_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_yet_another_listener",
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy),
        )
        .await?;

    let alice_yet_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_another_channel, "bob_yet_another_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(
            route![alice_yet_another_channel, child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.body());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!("Hello, Alice!", child_ctx.receive::<String>().await?.body());

    ctx.stop().await
}

#[ockam_macros::test]
async fn test_many_times_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier());

    let n = rand::random::<u8>() % 5 + 4;
    let mut channels: Vec<Address> = vec![];

    for i in 0..n {
        secure_channels
            .create_secure_channel_listener(
                ctx,
                &bob,
                i.to_string(),
                SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone()),
            )
            .await?;

        let channel_route = if i > 0 {
            route![channels.pop().unwrap(), i.to_string()]
        } else {
            route![i.to_string()]
        };

        let alice_channel = secure_channels
            .create_secure_channel(
                ctx,
                &alice,
                channel_route,
                SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
            )
            .await?;

        channels.push(alice_channel);
    }

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(
            route![channels.pop().unwrap(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.body());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!("Hello, Alice!", child_ctx.receive::<String>().await?.body());

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
async fn access_control__known_participant__should_pass_messages(ctx: &mut Context) -> Result<()> {
    let received_count = Arc::new(AtomicU8::new(0));
    let receiver = Receiver {
        received_count: received_count.clone(),
    };

    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let access_control = IdentityAccessControlBuilder::new_with_id(alice.identifier());
    WorkerBuilder::with_access_control(
        Arc::new(access_control),
        Arc::new(DenyAll),
        "receiver",
        receiver,
    )
    .start(ctx)
    .await?;

    secure_channels
        .create_secure_channel_listener(ctx, &bob, "listener", SecureChannelListenerOptions::new())
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["listener"],
            SecureChannelOptions::new().with_trust_policy(TrustEveryonePolicy),
        )
        .await?;

    let child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
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

    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let access_control = IdentityAccessControlBuilder::new_with_id(bob.identifier());
    WorkerBuilder::with_access_control(
        Arc::new(access_control),
        Arc::new(DenyAll),
        "receiver",
        receiver,
    )
    .start(ctx)
    .await?;

    secure_channels
        .create_secure_channel_listener(ctx, &bob, "listener", SecureChannelListenerOptions::new())
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["listener"],
            SecureChannelOptions::new().with_trust_policy(TrustEveryonePolicy),
        )
        .await?;

    let child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
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
    WorkerBuilder::with_access_control(
        Arc::new(access_control),
        Arc::new(DenyAll),
        "receiver",
        receiver,
    )
    .start(ctx)
    .await?;

    let child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(route!["receiver"], "Hello, Bob!".to_string())
        .await?;

    sleep(Duration::from_secs(1)).await;

    assert_eq!(received_count.load(Ordering::Relaxed), 0);

    ctx.stop().await
}
