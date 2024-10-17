use core::time::Duration;
use std::sync::atomic::{AtomicU8, Ordering};

use ockam_core::compat::sync::Arc;
use ockam_core::{
    route, Address, AllowAll, Any, DenyAll, Mailboxes, Result, Routed, SecureChannelLocalInfo,
    Worker, SECURE_CHANNEL_IDENTIFIER,
};
use ockam_identity::models::{CredentialSchemaIdentifier, Identifier};
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{
    DecryptionResponse, EncryptionRequest, EncryptionResponse, IdentityAccessControlBuilder,
    SecureChannelListenerOptions, SecureChannelOptions, SecureChannels, TrustEveryonePolicy,
    TrustIdentifierPolicy, Vault,
};
use ockam_node::{Context, MessageReceiveOptions, WorkerBuilder};
use ockam_vault::{
    SoftwareVaultForSecureChannels, SoftwareVaultForSigning, SoftwareVaultForVerifyingSignatures,
};

#[ockam_macros::test]
async fn test_channel(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.clone());

    let bob_options = SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy);
    let bob_listener = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_listener", bob_options)
        .await?;

    let alice_options = SecureChannelOptions::new().with_trust_policy(alice_trust_policy);
    let alice_channel = secure_channels
        .create_secure_channel(ctx, &alice, route!["bob_listener"], alice_options)
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.flow_controls()
        .add_consumer("child", bob_listener.flow_control_id());

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = SecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(Identifier::from(local_info.their_identifier()), alice);

    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.into_body()?);

    ctx.flow_controls()
        .add_consumer("child", alice_channel.flow_control_id());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = SecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(Identifier::from(local_info.their_identifier()), bob);

    assert_eq!("Hello, Alice!", msg.into_body()?);

    Ok(())
}

#[ockam_macros::test]
async fn test_channel_send_credentials(context: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let authority = identities_creation.create_identity().await?;

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let _bob_credential_1st = secure_channels
        .identities()
        .credentials()
        .credentials_creation()
        .issue_credential(
            &authority,
            &bob,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_bob", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    let bob_credential_2 = secure_channels
        .identities()
        .credentials()
        .credentials_creation()
        .issue_credential(
            &authority,
            &bob,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("bob_2", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    secure_channels
        .create_secure_channel_listener(
            context,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new()
                .with_authority(authority.clone())
                .with_credential(bob_credential_2)?,
        )
        .await?;

    let _alice_credential_1st = secure_channels
        .identities()
        .credentials()
        .credentials_creation()
        .issue_credential(
            &authority,
            &alice,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_alice", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    let alice_credential_2 = secure_channels
        .identities()
        .credentials()
        .credentials_creation()
        .issue_credential(
            &authority,
            &alice,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("alice_2", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    let _alice_channel = secure_channels
        .create_secure_channel(
            context,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new()
                .with_authority(authority.clone())
                .with_credential(alice_credential_2)?,
        )
        .await?;

    context.sleep(Duration::from_millis(250)).await;

    let alice_attributes = secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&alice, &authority)
        .await?
        .unwrap();

    //FIXME: only the last credential is kept around in the storage
    // assert_eq!(
    //     "true".as_bytes(),
    //     alice_attributes.attrs().get("is_alice").unwrap()
    // );
    assert_eq!(
        "true".as_bytes(),
        alice_attributes.attrs().get("alice_2".as_bytes()).unwrap()
    );
    assert!(alice_attributes.attrs().get("is_bob".as_bytes()).is_none());
    assert!(alice_attributes.attrs().get("bob_2".as_bytes()).is_none());

    let bob_attributes = secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&bob, &authority)
        .await?
        .unwrap();

    assert!(bob_attributes.attrs().get("is_alice".as_bytes()).is_none());
    assert!(bob_attributes.attrs().get("alice_2".as_bytes()).is_none());
    //FIXME: only the last credential is kept around in the storage
    // assert_eq!(
    //     "true".as_bytes(),
    //     bob_attributes.attrs().get("is_bob").unwrap()
    // );
    assert_eq!(
        "true".as_bytes(),
        bob_attributes.attrs().get("bob_2".as_bytes()).unwrap()
    );

    Ok(())
}

#[ockam_macros::test]
async fn test_channel_rejected_trust_policy(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_broken_trust_policy = TrustIdentifierPolicy::new(
        Identifier::try_from("Iabababababababababababababababababababababababababababababababab")
            .unwrap(),
    );

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().with_trust_policy(alice_broken_trust_policy),
        )
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_timeout(Duration::from_millis(500)),
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

    let result = child_ctx
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_millis(50)),
        )
        .await;

    assert!(result.is_err());

    Ok(())
}

#[ockam_macros::test]
async fn test_channel_send_multiple_messages_both_directions(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.clone());

    let bob_options = SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy);
    let sc_listener_flow_control_id = bob_options.spawner_flow_control_id();
    secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_listener", bob_options)
        .await?;

    let alice_options = SecureChannelOptions::new().with_trust_policy(alice_trust_policy);
    let sc_flow_control_id = alice_options.producer_flow_control_id();
    let alice_channel = secure_channels
        .create_secure_channel(ctx, &alice, route!["bob_listener"], alice_options)
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    for n in 0..50 {
        child_ctx
            .flow_controls()
            .add_consumer(child_ctx.address(), &sc_listener_flow_control_id);
        let payload = format!("Hello, Bob! {}", n);
        child_ctx
            .send(
                route![alice_channel.clone(), child_ctx.address()],
                payload.clone(),
            )
            .await?;

        let message = child_ctx.receive::<String>().await?;
        let return_route = message.return_route();
        assert_eq!(payload, message.into_body()?);

        child_ctx
            .flow_controls()
            .add_consumer(child_ctx.address(), &sc_flow_control_id);
        let payload = format!("Hello, Alice! {}", n);
        child_ctx.send(return_route, payload.clone()).await?;

        let message = child_ctx.receive::<String>().await?;
        assert_eq!(payload, message.into_body()?);
    }
    Ok(())
}

#[ockam_macros::test]
async fn test_channel_registry(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels
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
        .get_channel_by_encryptor_address(alice_channel.encryptor_address())
        .unwrap();

    assert!(alice_channel_data.is_initiator());
    assert_eq!(alice_channel_data.my_id(), &alice);
    assert_eq!(alice_channel_data.their_id(), &bob);

    let mut bob_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "bob",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.flow_controls()
        .add_consumer("bob", bob_listener.flow_control_id());

    ctx.send(
        route![alice_channel.clone(), "bob"],
        "Hello, Alice!".to_string(),
    )
    .await?;

    let msg = bob_ctx.receive::<String>().await?;
    let return_route = msg.return_route();

    assert_eq!("Hello, Alice!", msg.into_body()?);

    let bob_channel = return_route.next().unwrap().clone();

    let bob_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(&bob_channel)
        .unwrap();

    assert!(!bob_channel_data.is_initiator());
    assert_eq!(bob_channel_data.my_id(), &bob);
    assert_eq!(bob_channel_data.their_id(), &alice);

    Ok(())
}

#[ockam_macros::test]
async fn test_channel_api(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels
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

    ctx.flow_controls()
        .add_consumer("bob", bob_listener.flow_control_id());

    ctx.send(
        route![alice_channel.clone(), "bob"],
        "Hello, Alice!".to_string(),
    )
    .await?;

    let msg = bob_ctx.receive::<String>().await?;
    let return_route = msg.return_route();

    assert_eq!("Hello, Alice!", msg.into_body()?);

    let bob_channel = return_route.next().unwrap().clone();

    let alice_channel_data = secure_channels
        .secure_channel_registry()
        .get_channel_by_encryptor_address(alice_channel.encryptor_address())
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

    Ok(())
}

#[ockam_macros::test]
async fn test_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.clone());

    let bob_options =
        SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone());
    let bob_listener = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_listener", bob_options)
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    let bob_options_2 = SecureChannelListenerOptions::new()
        .as_consumer(bob_listener.flow_control_id())
        .with_trust_policy(bob_trust_policy);
    let bob_listener2 = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_another_listener", bob_options_2)
        .await?;

    let alice_options2 = SecureChannelOptions::new().with_trust_policy(alice_trust_policy);
    let alice_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_channel, "bob_another_listener"],
            alice_options2,
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.flow_controls()
        .add_consumer("child", bob_listener2.flow_control_id());

    child_ctx
        .send(
            route![alice_another_channel.clone(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.into_body()?);

    ctx.flow_controls()
        .add_consumer("child", alice_another_channel.flow_control_id());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!(
        "Hello, Alice!",
        child_ctx.receive::<String>().await?.into_body()?
    );

    Ok(())
}

#[ockam_macros::test]
async fn test_double_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.clone());

    let bob_options =
        SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone());
    let bob_listener = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_listener", bob_options)
        .await?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    let bob_options2 = SecureChannelListenerOptions::new()
        .as_consumer(bob_listener.flow_control_id())
        .with_trust_policy(bob_trust_policy.clone());
    let bob_listener2 = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_another_listener", bob_options2)
        .await?;

    let alice_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_channel, "bob_another_listener"],
            SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone()),
        )
        .await?;

    let bob_options3 = SecureChannelListenerOptions::new()
        .as_consumer(bob_listener2.flow_control_id())
        .with_trust_policy(bob_trust_policy);
    let bob_listener3 = secure_channels
        .create_secure_channel_listener(ctx, &bob, "bob_yet_another_listener", bob_options3)
        .await?;

    let alice_options3 = SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone());
    let alice_yet_another_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![alice_another_channel, "bob_yet_another_listener"],
            alice_options3,
        )
        .await?;

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.flow_controls()
        .add_consumer("child", bob_listener3.flow_control_id());

    child_ctx
        .send(
            route![alice_yet_another_channel.clone(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.into_body()?);

    ctx.flow_controls()
        .add_consumer("child", alice_yet_another_channel.flow_control_id());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!(
        "Hello, Alice!",
        child_ctx.receive::<String>().await?.into_body()?
    );

    Ok(())
}

#[ockam_macros::test]
async fn test_many_times_tunneled_secure_channel_works(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.clone());

    let n = rand::random::<u8>() % 5 + 4;
    let mut channels: Vec<Address> = vec![];
    let mut sc_flow_control_id = None;
    let mut sc_listener_flow_control_id = None;

    for i in 0..n {
        let options =
            SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy.clone());
        let options = match &sc_listener_flow_control_id {
            Some(flow_control_id) => options.as_consumer(flow_control_id),
            None => options,
        };
        sc_listener_flow_control_id = Some(options.spawner_flow_control_id());
        secure_channels
            .create_secure_channel_listener(ctx, &bob, i.to_string(), options)
            .await?;
        let mut route = route![i.to_string()];
        if let Some(last_channel) = channels.last() {
            route.modify().prepend(last_channel.clone());
        }

        let options = SecureChannelOptions::new().with_trust_policy(alice_trust_policy.clone());
        sc_flow_control_id = Some(options.producer_flow_control_id());
        let alice_channel = secure_channels
            .create_secure_channel(ctx, &alice, route, options)
            .await?;

        channels.push(alice_channel.encryptor_address().clone());
    }

    let mut child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    ctx.flow_controls()
        .add_consumer("child", &sc_listener_flow_control_id.unwrap());

    child_ctx
        .send(
            route![channels.last().unwrap().clone(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.into_body()?);

    ctx.flow_controls()
        .add_consumer("child", &sc_flow_control_id.unwrap());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;
    assert_eq!(
        "Hello, Alice!",
        child_ctx.receive::<String>().await?.into_body()?
    );
    Ok(())
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

    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let access_control = IdentityAccessControlBuilder::new_with_id(alice.clone());
    WorkerBuilder::new(receiver)
        .with_address("receiver")
        .with_incoming_access_control(access_control)
        .with_outgoing_access_control(DenyAll)
        .start(ctx)
        .await?;

    let bob_listener = secure_channels
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

    ctx.flow_controls()
        .add_consumer("receiver", bob_listener.flow_control_id());

    ctx.send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    assert_eq!(received_count.load(Ordering::Relaxed), 1);

    Ok(())
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

    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let access_control = IdentityAccessControlBuilder::new_with_id(bob.clone());
    WorkerBuilder::new(receiver)
        .with_address("receiver")
        .with_incoming_access_control(access_control)
        .with_outgoing_access_control(DenyAll)
        .start(ctx)
        .await?;

    let bob_listener = secure_channels
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

    ctx.flow_controls()
        .add_consumer("receiver", bob_listener.flow_control_id());

    ctx.send(route![alice_channel, "receiver"], "Hello, Bob!".to_string())
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    assert_eq!(received_count.load(Ordering::Relaxed), 0);

    Ok(())
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
        "Iabababababababababababababababababababababababababababababababab".try_into()?,
    );
    WorkerBuilder::new(receiver)
        .with_address("receiver")
        .with_incoming_access_control(access_control)
        .with_outgoing_access_control(DenyAll)
        .start(ctx)
        .await?;

    ctx.send(route!["receiver"], "Hello, Bob!".to_string())
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    assert_eq!(received_count.load(Ordering::Relaxed), 0);

    Ok(())
}

#[ockam_macros::test]
async fn test_channel_delete_ephemeral_keys(ctx: &mut Context) -> Result<()> {
    let alice_identity_vault = SoftwareVaultForSigning::create().await?;
    let alice_sc_vault = SoftwareVaultForSecureChannels::create().await?;
    let alice_vault = Vault::new(
        alice_identity_vault.clone(),
        alice_sc_vault.clone(),
        SoftwareVaultForSigning::create().await?,
        SoftwareVaultForVerifyingSignatures::create(),
    );

    let bob_identity_vault = SoftwareVaultForSigning::create().await?;
    let bob_sc_vault = SoftwareVaultForSecureChannels::create().await?;
    let bob_vault = Vault::new(
        bob_identity_vault.clone(),
        bob_sc_vault.clone(),
        SoftwareVaultForSigning::create().await?,
        SoftwareVaultForVerifyingSignatures::create(),
    );

    let secure_channels_alice = SecureChannels::builder()
        .await?
        .with_vault(alice_vault)
        .build();
    let secure_channels_bob = SecureChannels::builder()
        .await?
        .with_vault(bob_vault)
        .build();

    let identities_creation_alice = secure_channels_alice.identities().identities_creation();
    let identities_creation_bob = secure_channels_bob.identities().identities_creation();

    assert_eq!(alice_identity_vault.number_of_keys().await?, 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_static_x25519_secrets().await?, 0);
    assert_eq!(bob_identity_vault.number_of_keys().await?, 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_static_x25519_secrets().await?, 0);

    let alice = identities_creation_alice.create_identity().await?;
    assert_eq!(alice_identity_vault.number_of_keys().await?, 1);
    assert_eq!(alice_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_static_x25519_secrets().await?, 0);

    secure_channels_alice
        .identities()
        .purpose_keys()
        .purpose_keys_creation()
        .create_secure_channel_purpose_key(&alice)
        .await?;
    assert_eq!(alice_identity_vault.number_of_keys().await?, 1);
    assert_eq!(alice_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_static_x25519_secrets().await?, 1);

    let bob = identities_creation_bob.create_identity().await?;
    secure_channels_bob
        .identities()
        .purpose_keys()
        .purpose_keys_creation()
        .create_secure_channel_purpose_key(&bob)
        .await?;

    secure_channels_bob
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;
    assert_eq!(bob_identity_vault.number_of_keys().await?, 1);
    assert_eq!(bob_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_static_x25519_secrets().await?, 1);

    secure_channels_alice
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;
    ctx.sleep(Duration::from_millis(250)).await;

    // k1, k2 and purpose key should exist
    assert_eq!(alice_identity_vault.number_of_keys().await?, 1);
    assert_eq!(alice_sc_vault.number_of_ephemeral_aead_secrets(), 2);
    assert_eq!(alice_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_static_x25519_secrets().await?, 1);

    assert_eq!(bob_identity_vault.number_of_keys().await?, 1);
    assert_eq!(bob_sc_vault.number_of_ephemeral_aead_secrets(), 2);
    assert_eq!(bob_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_static_x25519_secrets().await?, 1);

    ctx.stop().await?;

    // when the channel is closed only purpose key should be left
    assert_eq!(alice_identity_vault.number_of_keys().await?, 1);
    assert_eq!(alice_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(alice_sc_vault.number_of_static_x25519_secrets().await?, 1);

    assert_eq!(bob_identity_vault.number_of_keys().await?, 1);
    assert_eq!(bob_sc_vault.number_of_ephemeral_aead_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_buffer_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_ephemeral_x25519_secrets(), 0);
    assert_eq!(bob_sc_vault.number_of_static_x25519_secrets().await?, 1);

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn should_stop_encryptor__and__decryptor__in__secure_channel(
    ctx: &mut Context,
) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let _bob_listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    ctx.sleep(Duration::from_millis(250)).await;

    let sc_list = secure_channels.secure_channel_registry().get_channel_list();
    assert_eq!(sc_list.len(), 2);

    let channel1 = sc_list[0].clone();
    let channel2 = sc_list[1].clone();

    // This will stop both ends of the channel
    secure_channels
        .stop_secure_channel(ctx, channel1.encryptor_messaging_address())
        .await?;

    ctx.sleep(Duration::from_millis(250)).await;

    assert_eq!(
        secure_channels
            .secure_channel_registry()
            .get_channel_list()
            .len(),
        0
    );

    let workers = ctx.list_workers().await?;
    assert!(!workers.contains(channel1.decryptor_messaging_address()));
    assert!(!workers.contains(channel1.encryptor_messaging_address()));
    assert!(!workers.contains(channel2.decryptor_messaging_address()));
    assert!(!workers.contains(channel2.encryptor_messaging_address()));

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn address_metadata__encryptor__should_be_terminal(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let _bob_listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let sc = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let meta = ctx
        .find_terminal_address(route!["app", sc.clone(), "test"])
        .await?
        .unwrap();

    assert_eq!(meta.address, sc.into());
    assert_eq!(
        meta.metadata.attributes,
        vec![(SECURE_CHANNEL_IDENTIFIER.to_string(), hex::encode(bob.0))]
    );
    assert!(meta.metadata.is_terminal);

    Ok(())
}
