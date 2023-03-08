use ockam::access_control::AllowAll;
use ockam::access_control::IdentityIdAccessControl;
use ockam::identity::credential_issuer::CredentialIssuer;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Context, Result, TcpListenerTrustOptions, TcpTransport};

/// This node starts a temporary credential issuer accessible via TCP on localhost:5000
///
/// In a real-life scenario this node would be an "Authority", a node holding
/// attributes for a number of identities and able to issue credentials signed with its own key.
///
/// The process by which we declare to that Authority which identity holds which attributes is an
/// enrollment process and would be driven by an "enroller node".
/// For the simplicity of the example provided here we preload the credential issues node with some existing attributes
/// for both Alice's and Bob's identities.
///
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:5000", TcpListenerTrustOptions::new()).await?;

    // Create a CredentialIssuer which stores attributes for Alice and Bob, knowing their identity
    let issuer = CredentialIssuer::create(&ctx).await?;
    let alice = "Pe92f183eb4c324804ef4d62962dea94cf095a265d4d28500c34e1a4e0d5ef638".try_into()?;
    let bob = "Pada09e0f96e56580f6a0cb54f55ecbde6c973db6732e30dfb39b178760aed041".try_into()?;

    issuer.put_attribute_value(&alice, "name", "alice").await?;
    issuer.put_attribute_value(&bob, "name", "bob").await?;

    // Start a secure channel listener that alice and bob can use to retrieve their credential
    issuer
        .identity()
        .create_secure_channel_listener("issuer_listener", TrustEveryonePolicy)
        .await?;
    println!("created a secure channel listener");

    ctx.start_worker(
        "issuer",
        issuer,
        IdentityIdAccessControl::new(vec![alice, bob]),
        AllowAll,
    )
    .await?;

    Ok(())
}
