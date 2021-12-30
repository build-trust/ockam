use credentials_example::{
    read_line, DOOR_LISTENER_ADDRESS, DOOR_TCP_ADDRESS, DOOR_WORKER_ADDRESS,
    OFFICE_LISTENER_ADDRESS, OFFICE_TCP_ADDRESS,
};
use ockam::{
    credential_attribute_values, credential_type, route, Context, CredentialProtocol, Entity,
    EntityIdentifier, Identity, Result, TcpTransport, TrustEveryonePolicy,
    TrustIdentifierPolicy, Vault, TCP,
};
use std::convert::TryFrom;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx).await?;
    let mut entity = Entity::create(&ctx, &vault).await?;

    println!("Door id: {}", entity.identifier().await?);

    println!("Enter Office id: ");
    let office_id = read_line();
    let office_id = EntityIdentifier::try_from(office_id.as_str())?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(OFFICE_TCP_ADDRESS).await?;

    // Just to get office's profile
    let _office_channel = entity.create_secure_channel(
        route![(TCP, OFFICE_TCP_ADDRESS), OFFICE_LISTENER_ADDRESS],
        TrustIdentifierPolicy::new(office_id.clone()),
    ).await?;

    entity.create_secure_channel_listener(DOOR_LISTENER_ADDRESS, TrustEveryonePolicy).await?;

    tcp.listen(DOOR_TCP_ADDRESS).await?;

    // TODO: Add listener
    let res = entity.verify_credential(
        DOOR_WORKER_ADDRESS.into(),
        &office_id,
        credential_type!["TYPE_ID"; "door_id", (Number, "can_open_door")],
        credential_attribute_values![entity.identifier().await?.to_string(), 1],
    ).await?;
    assert!(res);

    // TODO: Add credential expiration
    // TODO: Store information that holder posses the credential
    // TODO: Add actual Door controlling Worker

    println!("Door is opened!");

    Ok(())
}
