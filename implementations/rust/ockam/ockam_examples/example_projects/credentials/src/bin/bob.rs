use lib::{door_schema, read_line};
use ockam::{
    route, Context, CredentialProtocol, Entity, Identity, ProfileIdentifier, Result,
    SecureChannels, TcpTransport, TCP,
};

const OFFICE_TCP_ADDRESS: &str = "127.0.0.1:4222";
const OFFICE_LISTENER_ADDRESS: &str = "office_listener";
const OFFICE_ISSUER_ADDRESS: &str = "office_issuer";
const DOOR_TCP_ADDRESS: &str = "127.0.0.1:5333";
const DOOR_LISTENER_ADDRESS: &str = "door_listener";
const DOOR_WORKER_ADDRESS: &str = "app";

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Bob is a Holder of a credential, allowing him to open doors.
    let mut entity = Entity::create(&ctx)?;
    println!("Bob id: {}", entity.identifier()?.to_external());

    println!("Enter Office id: ");
    let office_id = read_line();
    let office_id = ProfileIdentifier::from_external(office_id.as_str())?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(OFFICE_TCP_ADDRESS).await?;

    // TODO: Should we support change TrustPolicy on existing SecureChannelListener
    let office_channel = entity.create_secure_channel(
        route![(TCP, OFFICE_TCP_ADDRESS), OFFICE_LISTENER_ADDRESS],
        // FIXME IdentifierTrustPolicy::new(office_id.clone()),
    )?;

    let credential = entity.acquire_credential(
        route![office_channel, OFFICE_ISSUER_ADDRESS],
        &office_id,
        door_schema(),
    )?;

    println!("Bob got credential!");

    println!("Enter Door id: ");
    let door_id = read_line();
    let door_id = ProfileIdentifier::from_external(door_id.as_str())?;

    tcp.connect(DOOR_TCP_ADDRESS).await?;

    let door_channel = entity.create_secure_channel(
        route![(TCP, DOOR_TCP_ADDRESS), DOOR_LISTENER_ADDRESS],
        // FIXME IdentifierTrustPolicy::new(door_id.clone()),
    )?;

    entity.prove_credential(
        route![door_channel, DOOR_WORKER_ADDRESS],
        &door_id,
        credential,
    )?;

    // TODO: Send actual payload

    ctx.stop().await?;

    Ok(())
}
