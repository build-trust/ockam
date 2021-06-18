use lib::{door_schema, read_line};
use ockam::{
    Context, CredentialIssuer, CredentialProtocol, Entity, IdentifierTrustPolicy, KeyAttributes,
    Profile, ProfileIdentifier, ProfileIdentity, ProfileSecrets, Result, TcpTransport,
};

const OFFICE_TCP_ADDRESS: &str = "127.0.0.1:4222";
const OFFICE_LISTENER_ADDRESS: &str = "office_listener";

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(OFFICE_TCP_ADDRESS).await?;

    // The Office is an Issuer of credentials. In this case, permission to open doors.
    let mut entity = Entity::create(&ctx).await?;
    entity.create_key(KeyAttributes::new(Profile::SIGNING.to_string()), None)?;

    println!(
        "Office id: {}",
        entity.identifier()?.to_string_representation()
    );
    println!(
        "Office pubkey: {}",
        hex::encode(entity.get_signing_public_key()?)
    );

    println!("Enter Bob id: ");
    let bob_id = read_line();
    let bob_id = ProfileIdentifier::from_string_representation(bob_id);

    entity
        .create_secure_channel_listener(
            OFFICE_LISTENER_ADDRESS,
            IdentifierTrustPolicy::new(bob_id.clone()),
        )
        .await?;

    // TODO: Turn into a worker
    entity.issue_credential(&bob_id, door_schema()).await?;

    ctx.stop().await?;

    Ok(())
}
