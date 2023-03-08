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
    // We preload Bob's vault with a change history and secret key corresponding to the identity identifier
    // Pada09e0f96e56580f6a0cb54f55ecbde6c973db6732e30dfb39b178760aed041
    // which is an identifier known to the credential issuer, with some preset attributes
    let vault = Vault::create();
    let identity_history = "01ed8a5b1303f975c1296c990d1bd3c1946cfef328de20531e3511ec5604ce0dd9000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e8c328bc0cc07a374762091d037e69c36fdd4d2e1a651abd4d43a1362d3f800503010140a349968063d7337d0c965969fa9c640824c01a6d37fe130d4ab963b0271b9d5bbf0923faa5e27f15359554f94f08676df01b99d997944e4feaf0caaa1189480e";
    let secret = "5b2b3f2abbd1787704d8f8b363529f8e2d8f423b6dd4b96a2c462e4f0e04ee18";
    let bob = Identity::create_identity_with_history(&ctx, vault, identity_history, secret).await?;

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
