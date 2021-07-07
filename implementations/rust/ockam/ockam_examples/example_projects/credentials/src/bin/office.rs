use lib::{door_schema, read_line};
use ockam::{
    Context, CredentialProtocol, Entity, Identity, Issuer, Profile, ProfileIdentifier, Result,
    SecureChannels, TcpTransport,
};

const OFFICE_TCP_ADDRESS: &str = "127.0.0.1:4222";
const OFFICE_LISTENER_ADDRESS: &str = "office_listener";
const OFFICE_ISSUER_ADDRESS: &str = "office_issuer";

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(OFFICE_TCP_ADDRESS).await?;

    // The Office is an Issuer of credentials. In this case, permission to open doors.
    let mut entity = Entity::create(&ctx)?;
    entity.create_key(Profile::CREDENTIALS_ISSUE)?;

    println!("Office id: {}", entity.identifier()?.to_external());
    println!(
        "Office pubkey: {}",
        hex::encode(entity.get_signing_public_key()?)
    );

    println!("Enter Bob id: ");
    let bob_id = read_line();
    let bob_id = ProfileIdentifier::from_external(bob_id.as_str())?;

    entity.start_credential_issuer_worker(OFFICE_ISSUER_ADDRESS, &bob_id, door_schema())?;

    entity.create_secure_channel_listener(
        OFFICE_LISTENER_ADDRESS,
        // FIXME IdentifierTrustPolicy::new(bob_id.clone()),
    )?;

    Ok(())
}
