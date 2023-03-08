use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential_issuer::{CredentialIssuerApi, CredentialIssuerClient};
use ockam::identity::{Identity, SecureChannelTrustOptions, TrustEveryonePolicy};
use ockam::sessions::Sessions;
use ockam::{route, vault::Vault, Context, Result, TcpConnectionTrustOptions, TcpTransport};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity representing Alice
    // We preload Alice's vault with a change history and secret key corresponding to the identity identifier
    // Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638
    // which is an identifier known to the credential issuer, with some preset attributes
    let vault = Vault::create();

    let identity_history = "01dcf392551f796ef1bcb368177e53f9a5875a962f67279259207d24a01e690721000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020a0d205f09cab9a9467591fcee560429aab1215d8136e5c985a6b7dc729e6f08203010140b098463a727454c0e5292390d8f4cbd4dd0cae5db95606832f3d0a138936487e1da1489c40d8a0995fce71cc1948c6bcfd67186467cdd78eab7e95c080141505";
    let secret = "41b6873b20d95567bf958e6bab2808e9157720040882630b1bb37a72f4015cd2";
    let alice = Identity::create_identity_with_history(&ctx, vault, identity_history, secret).await?;

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
    let storage = AuthenticatedAttributeStorage::new(alice.authenticated_storage().clone());
    alice
        .present_credential_mutual(
            route![channel.clone(), "credential_exchange"],
            vec![&issuer.public_identity().await?],
            Arc::new(storage),
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
