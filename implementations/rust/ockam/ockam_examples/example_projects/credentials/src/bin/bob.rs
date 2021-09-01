use credentials_example::{
    read_line, DOOR_LISTENER_ADDRESS, DOOR_TCP_ADDRESS, DOOR_WORKER_ADDRESS, OFFICE_ISSUER_ADDRESS,
    OFFICE_LISTENER_ADDRESS, OFFICE_TCP_ADDRESS,
};
use ockam::{
    credential_attribute_values, credential_type, reveal_attributes, route, Context,
    CredentialProtocol, Entity, EntityIdentifier, Identity, Result, SecureChannels, TcpTransport,
    TrustIdentifierPolicy, Vault, TCP,
};
use std::convert::TryFrom;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx)?;
    let mut entity = Entity::create(&ctx, &vault)?;
    println!("Bob id: {}", entity.identifier()?);

    println!("Enter Office id: ");
    let office_id = read_line();
    let office_id = EntityIdentifier::try_from(office_id.as_str())?;

    println!("Enter Door id: ");
    let door_id = read_line();
    let door_id = EntityIdentifier::try_from(door_id.as_str())?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(OFFICE_TCP_ADDRESS).await?;

    let office_channel = entity.create_secure_channel(
        route![(TCP, OFFICE_TCP_ADDRESS), OFFICE_LISTENER_ADDRESS],
        TrustIdentifierPolicy::new(office_id.clone()),
    )?;

    let credential = entity.acquire_credential(
        route![office_channel, OFFICE_ISSUER_ADDRESS],
        &office_id,
        credential_type!["TYPE_ID"; "door_id", (Number, "can_open_door")],
        credential_attribute_values![door_id.clone().to_string(), 1],
    )?;

    println!("Bob got credential!");

    tcp.connect(DOOR_TCP_ADDRESS).await?;

    let door_channel = entity.create_secure_channel(
        route![(TCP, DOOR_TCP_ADDRESS), DOOR_LISTENER_ADDRESS],
        TrustIdentifierPolicy::new(door_id.clone()),
    )?;

    entity.present_credential(
        route![door_channel, DOOR_WORKER_ADDRESS],
        credential,
        reveal_attributes!["door_id", "can_open_door"],
    )?;

    println!("Bob presented credential!");

    // TODO: Send actual payload

    Ok(())
}
