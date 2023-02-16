use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::authority::{AuthorityApi, AuthorityClient};
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport
    TcpTransport::create(&ctx).await?;

    // Create an Identity to represent Alice
    let alice = Identity::create(&ctx, &Vault::create()).await?;

    // Create a client to the Authority
    let authority_route = route![(TCP, "127.0.0.1:5000"), "authority"];
    let authority = AuthorityClient::new(&ctx, authority_route).await?;

    // Get a credential for Alice
    let credential = authority
        .get_attribute_credential(alice.identifier(), "name", "alice")
        .await?;
    println!("got a credential from the authority\n{credential}");
    alice.set_credential(credential).await;

    // Create a secure channel to Bob's node
    let channel = alice
        .create_secure_channel(route![(TCP, "127.0.0.1:4000"), "bob_listener"], TrustEveryonePolicy)
        .await?;
    println!("created a secure channel at {channel:?}");

    // Send Alice credentials over the secure channel
    alice
        .present_credential_mutual(
            route![channel.clone(), "credential_exchange"],
            vec![&authority.public_identity().await?],
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
