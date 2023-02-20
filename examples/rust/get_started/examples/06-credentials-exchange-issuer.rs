use ockam::identity::credential_issuer::CredentialIssuer;
use ockam::identity::TrustEveryonePolicy;
use ockam::{Context, IdentityIdAccessControl, TcpTransport};
use ockam_core::{AllowAll, Result};

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
    tcp.listen("127.0.0.1:5000").await?;

    // Create a CredentialIssuer which stores attributes for Alice and Bob, knowing their identity
    let issuer = CredentialIssuer::create(&ctx).await?;
    let alice = "P529d43ac7b01e23d3818d00e083508790bfe8825714644b98134db6c1a7a6602".try_into()?;
    let bob = "P0189a2aec3799fe9d0dc0f982063022b697f18562a403eb46fa3d32be5bd31f8".try_into()?;

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
