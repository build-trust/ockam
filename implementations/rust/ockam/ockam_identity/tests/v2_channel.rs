use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll, Mailboxes, Result};
use ockam_identity::v2::secure_channels::secure_channels;
use ockam_identity::v2::{
    IdentitySecureChannelLocalInfo, Purpose, SecureChannelListenerOptions, SecureChannelOptions,
    TrustIdentifierPolicy,
};
use ockam_node::Context;

#[ockam_macros::test]
async fn test_channel(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier().clone());
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier().clone());

    let alice_purpose_key = secure_channels
        .identities()
        .purpose_keys()
        .create_purpose_key(alice.identifier(), Purpose::SecureChannel)
        .await?;
    let bob_purpose_key = secure_channels
        .identities()
        .purpose_keys()
        .create_purpose_key(bob.identifier(), Purpose::SecureChannel)
        .await?;

    let bob_options = SecureChannelListenerOptions::new().with_trust_policy(bob_trust_policy);
    let bob_listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            bob.identifier(),
            bob_purpose_key,
            "bob_listener",
            bob_options,
        )
        .await?;

    let alice_options = SecureChannelOptions::new().with_trust_policy(alice_trust_policy);
    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            alice.identifier(),
            alice_purpose_key,
            route!["bob_listener"],
            alice_options,
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
        .add_consumer("child", bob_listener.flow_control_id());

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.address()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(&local_info.their_identity_id(), alice.identifier());

    let return_route = msg.return_route();
    assert_eq!("Hello, Bob!", msg.body());

    ctx.flow_controls()
        .add_consumer("child", alice_channel.flow_control_id());

    child_ctx
        .send(return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
    assert_eq!(&local_info.their_identity_id(), bob.identifier());

    assert_eq!("Hello, Alice!", msg.body());

    ctx.stop().await
}
