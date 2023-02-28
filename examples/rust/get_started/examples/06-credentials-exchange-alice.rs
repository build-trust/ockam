use hello_ockam::create_identity_with_secret;
use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential_issuer::{CredentialIssuerApi, CredentialIssuerClient};
use ockam::identity::TrustEveryonePolicy;
use ockam::{route, vault::Vault, Context, Result, TcpTransport};

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
    let alice = create_identity_with_secret(&ctx, vault, &key_id, secret).await?;

    // Create a client to the credential issuer
    let issuer_connection = tcp.connect("127.0.0.1:5000").await?;
    let issuer_route = route![issuer_connection, "issuer_listener"];
    let issuer = CredentialIssuerClient::new(&ctx, &alice, issuer_route).await?;

    // Get a credential for Alice (this is done via a secure channel)
    let credential = issuer.get_credential(alice.identifier()).await?.unwrap();
    println!("got a credential from the issuer\n{credential}");
    alice.set_credential(credential).await;

    // Create a secure channel to Bob's node
    let bob_connection = tcp.connect("127.0.0.1:4000").await?;
    let channel = alice
        .create_secure_channel(route![bob_connection, "bob_listener"], TrustEveryonePolicy)
        .await?;
    println!("created a secure channel at {channel:?}");

    // Send Alice credentials over the secure channel
    alice
        .present_credential_mutual(
            route![channel.clone(), "credential_exchange"],
            vec![&issuer.public_identity().await?],
            &AuthenticatedAttributeStorage::new(alice.authenticated_storage().clone()),
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
