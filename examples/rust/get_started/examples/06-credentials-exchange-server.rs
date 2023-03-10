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
    let vault = Vault::create();

    // Create an Identity representing the server
    // Load an identity corresponding to the following public identifier
    // Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638
    //
    // We're hard coding this specific identity because its public identifier is known
    // to the credential issuer as a member of the production cluster.
    let identity_history = "01ed8a5b1303f975c1296c990d1bd3c1946cfef328de20531e3511ec5604ce0dd9000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e8c328bc0cc07a374762091d037e69c36fdd4d2e1a651abd4d43a1362d3f800503010140a349968063d7337d0c965969fa9c640824c01a6d37fe130d4ab963b0271b9d5bbf0923faa5e27f15359554f94f08676df01b99d997944e4feaf0caaa1189480e";
    let secret = "5b2b3f2abbd1787704d8f8b363529f8e2d8f423b6dd4b96a2c462e4f0e04ee18";
    let server = Identity::create_identity_with_history(&ctx, vault, identity_history, secret).await?;
    let store = server.authenticated_storage();

    // Connect with the credential issuer and authenticate using the latest private
    // key of this program's hardcoded identity.
    //
    // The credential issuer already knows the public identifier of this identity
    // as a member of the production cluster so it returns a signed credential
    // attesting to that knowledge.
    let sessions = Sessions::default();
    let session_id = sessions.generate_session_id();
    let issuer_tcp_trust_options = TcpConnectionTrustOptions::new().with_session(&sessions, &session_id);
    let issuer_connection = tcp.connect("127.0.0.1:5000", issuer_tcp_trust_options).await?;
    let issuer_trust_options = SecureChannelTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_ciphertext_session(&sessions, &session_id);
    let issuer_channel = server
        .create_secure_channel(route![issuer_connection, "secure"], issuer_trust_options)
        .await?;
    let issuer = CredentialIssuerClient::new(&ctx, route![issuer_channel]).await?;
    let credential = issuer.get_credential(server.identifier()).await?.unwrap();
    println!("Credential:\n{credential}");
    server.set_credential(credential).await;

    // Start an echoer worker that will only accept incoming requests from
    // identities that have authenticated credentials issued by the above credential
    // issuer. These credentials must also attest that requesting identity is
    // a member of the production cluster.
    let allow_production = AbacAccessControl::create(store.clone(), "cluster", "production");
    ctx.start_worker("echoer", Echoer, allow_production, AllowAll).await?;

    // Start a worker which will receive credentials sent by the client and issued by the issuer node
    let issuer_identity = issuer.public_identity().await?;
    let storage = Arc::new(AuthenticatedAttributeStorage::new(store.clone()));
    server
        .start_credential_exchange_worker(vec![issuer_identity], "credentials", true, storage)
        .await?;

    // Start a secure channel listener that only allows channels with
    // authenticated identities.
    let listener_session_id = sessions.generate_session_id();
    let trust_options = SecureChannelListenerTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_session(&sessions, &listener_session_id);
    server.create_secure_channel_listener("secure", trust_options).await?;

    // Create a TCP listener and wait for incoming connections
    let tcp_listener_trust_options = TcpListenerTrustOptions::new().with_session(&sessions, &listener_session_id);
    tcp.listen("127.0.0.1:4000", tcp_listener_trust_options).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
