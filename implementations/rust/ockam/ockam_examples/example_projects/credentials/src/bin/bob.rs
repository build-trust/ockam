use lib::{door_schema, read_line};
use ockam::{
    route, Context, CredentialProtocol, Entity, IdentifierTrustPolicy, ProfileIdentifier,
    ProfileIdentity, Result, TcpTransport, TCP,
};

const OFFICE_TCP_ADDRESS: &str = "127.0.0.1:4222";
const OFFICE_LISTENER_ADDRESS: &str = "office_listener";
const OFFICE_ISSUER_ADDRESS: &str = "app";
const DOOR_TCP_ADDRESS: &str = "127.0.0.1:5333";
const DOOR_LISTENER_ADDRESS: &str = "door_listener";
const DOOR_WORKER_ADDRESS: &str = "app";

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Bob is a Holder of a credential, allowing him to open doors.
    let mut entity = Entity::create(&ctx).await?;
    println!(
        "Bob id: {}",
        entity.identifier()?.to_string_representation()
    );

    println!("Enter Office id: ");
    let office_id = read_line();
    let office_id = ProfileIdentifier::from_string_representation(office_id);

    println!("Enter Door id: ");
    let door_id = read_line();
    let door_id = ProfileIdentifier::from_string_representation(door_id);

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(OFFICE_TCP_ADDRESS).await?;

    // TODO: Should we support change TrustPolicy on existing SecureChannelListener
    let office_channel = entity
        .create_secure_channel(
            route![(TCP, OFFICE_TCP_ADDRESS), OFFICE_LISTENER_ADDRESS],
            IdentifierTrustPolicy::new(office_id.clone()),
        )
        .await?;

    let credential = entity
        .acquire_credential(
            route![office_channel, OFFICE_ISSUER_ADDRESS],
            &office_id,
            door_schema(),
        )
        .await
        .unwrap();

    println!("Bob got credential!");

    tcp.connect(DOOR_TCP_ADDRESS).await?;

    let door_channel = entity
        .create_secure_channel(
            route![(TCP, DOOR_TCP_ADDRESS), DOOR_LISTENER_ADDRESS],
            IdentifierTrustPolicy::new(door_id.clone()),
        )
        .await?;

    entity
        .prove_credential(
            route![door_channel, DOOR_WORKER_ADDRESS],
            &door_id,
            credential,
        )
        .await?;

    // TODO: Send actual payload

    ctx.stop().await?;

    Ok(())
}
