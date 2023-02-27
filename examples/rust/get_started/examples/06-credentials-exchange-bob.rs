use std::sync::Arc;
// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.
use hello_ockam::Echoer;
use ockam::abac::AbacAccessControl;
use ockam::access_control::AllowAll;
use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential_issuer::{CredentialIssuerApi, CredentialIssuerClient};
use ockam::identity::{Identity, SecureChannelListenerTrustOptions, SecureChannelTrustOptions, TrustEveryonePolicy};
use ockam::sessions::Sessions;
use ockam::{route, vault::Vault, Context, Result, TcpConnectionTrustOptions, TcpListenerTrustOptions, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity representing Bob
    // We preload Bob's vault with a secret key corresponding to the identity identifier
    // P0189a2aec3799fe9d0dc0f982063022b697f18562a403eb46fa3d32be5bd31f8
    // which is an identifier known to the credential issuer, with some preset attributes
    let vault = Vault::create();
    let key_id = "0189a2aec3799fe9d0dc0f982063022b697f18562a403eb46fa3d32be5bd31f8".to_string();
    let secret = "08ddb4458a53d5493eac7e9941a1b0d06896efa2d1efac8cf225ee1ccb824458";
    let bob = Identity::create_identity_with_secret(&ctx, vault, &key_id, secret).await?;

    // Create a client to a credential issuer
    let sessions = Sessions::default();
    let session_id = sessions.generate_session_id();
    let issuer_tcp_trust_options = TcpConnectionTrustOptions::new().with_session(&sessions, &session_id);
    let issuer_connection = tcp.connect("127.0.0.1:5000", issuer_tcp_trust_options).await?;
    let issuer_trust_options = SecureChannelTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_ciphertext_session(&sessions, &session_id);
    let issuer_channel = bob
        .create_secure_channel(route![issuer_connection, "issuer_listener"], issuer_trust_options)
        .await?;
    let issuer = CredentialIssuerClient::new(&ctx, route![issuer_channel]).await?;

    // Get a credential for Bob (this is done via a secure channel)
    let credential = issuer.get_credential(bob.identifier()).await?.unwrap();
    println!("got a credential from the issuer\n{credential}");
    bob.set_credential(credential).await;

    // Start a worker which will receive credentials sent by Alice and issued by the issuer node
    let issuer_identity = issuer.public_identity().await?;
    let storage = AuthenticatedAttributeStorage::new(bob.authenticated_storage().clone());
    bob.start_credential_exchange_worker(vec![issuer_identity], "credential_exchange", true, Arc::new(storage))
        .await?;

    // Create a secure channel listener to allow Alice to create a secure channel to Bob's node
    let listener_session_id = sessions.generate_session_id();
    let secure_channel_listener_trust_options = SecureChannelListenerTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_session(&sessions, &listener_session_id);
    bob.create_secure_channel_listener("bob_listener", secure_channel_listener_trust_options)
        .await?;
    println!("created a secure channel listener");

    // Start an echoer service which will only allow subjects with name = alice
    let alice_only = AbacAccessControl::create(bob.authenticated_storage().clone(), "name", "alice");
    ctx.start_worker("echoer", Echoer, alice_only, AllowAll).await?;

    // Create a TCP listener and wait for incoming connections
    let tcp_listener_trust_options = TcpListenerTrustOptions::new().with_session(&sessions, &listener_session_id);
    tcp.listen("127.0.0.1:4000", tcp_listener_trust_options).await?;
    println!("created a TCP listener");

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
