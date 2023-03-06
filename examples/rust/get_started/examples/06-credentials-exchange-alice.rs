use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential_issuer::{CredentialIssuerApi, CredentialIssuerClient};
use ockam::identity::{Identity, SecureChannelTrustOptions, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpConnectionTrustOptions, TcpTransport};
use ockam_core::sessions::Sessions;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity representing Alice
    // We preload Alice's vault with a secret key corresponding to the identity identifier
    // P529d43ac7b01e23d3818d00e083508790bfe8825714644b98134db6c1a7a6602
    // which is an identifier known to the credential issuer, with some preset attributes
    let vault = Vault::create();
    let key_id = "529d43ac7b01e23d3818d00e083508790bfe8825714644b98134db6c1a7a6602".to_string();
    let secret = "acaf50c540be1494d67aaad78aca8d22ac62c4deb4fb113991a7b30a0bd0c757";
    let alice = Identity::create_identity_with_secret(&ctx, vault, &key_id, secret).await?;

    // Create a client to the credential issuer
    let sessions = Sessions::default();
    let issuer_session_id = sessions.generate_session_id();
    let issuer_tcp_trust_options = TcpConnectionTrustOptions::new().with_session(&sessions, &issuer_session_id);
    let issuer_connection = tcp.connect("127.0.0.1:5000", issuer_tcp_trust_options).await?;
    let issuer_trust_options = SecureChannelTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_ciphertext_session(&sessions, &issuer_session_id);
    let issuer_channel = alice
        .create_secure_channel(route![issuer_connection, "issuer_listener"], issuer_trust_options)
        .await?;
    let issuer = CredentialIssuerClient::new(&ctx, route![issuer_channel]).await?;

    // Get a credential for Alice (this is done via a secure channel)
    let credential = issuer.get_credential(alice.identifier()).await?.unwrap();
    println!("got a credential from the issuer\n{credential}");
    alice.set_credential(credential).await;

    // Create a secure channel to Bob's node
    let bob_session_id = sessions.generate_session_id();
    let bob_tcp_trust_options = TcpConnectionTrustOptions::new().with_session(&sessions, &bob_session_id);
    let bob_connection = tcp.connect("127.0.0.1:4000", bob_tcp_trust_options).await?;
    let channel_trust_options = SecureChannelTrustOptions::new()
        .with_trust_policy(TrustEveryonePolicy)
        .with_ciphertext_session(&sessions, &bob_session_id);
    let channel = alice
        .create_secure_channel(route![bob_connection, "bob_listener"], channel_trust_options)
        .await?;
    println!("created a secure channel at {channel:?}");

    // Send Alice credentials over the secure channel
    alice
        .present_credential_mutual(
            route![channel.clone(), "credential_exchange"],
            vec![&issuer.public_identity().await?],
            &AuthenticatedAttributeStorage::new(alice.authenticated_storage().clone()),
            None,
        )
        .await?;
    println!("exchange done!");

    // The echoer service should now be accessible to Alice because she
    // presented the right credentials to Bob
    let received: String = ctx
        .send_and_receive(route![channel, "echoer"], "Hello!".to_string())
        .await?;
    println!("{received}");

    ctx.stop().await
}
