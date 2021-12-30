use credentials_example::{
    read_line, OFFICE_ISSUER_ADDRESS, OFFICE_LISTENER_ADDRESS, OFFICE_TCP_ADDRESS,
};
use ockam::{
    credential_type, Context, CredentialProtocol, Entity, EntityIdentifier, Identity, Profile,
    Result, TcpTransport, TrustEveryonePolicy, TrustIdentifierPolicy, Vault,
};
use std::convert::TryFrom;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(OFFICE_TCP_ADDRESS).await?;

    // The Office is an Issuer of credentials. In this case, permission to open doors.
    let vault = Vault::create(&ctx).await?;
    let mut entity = Entity::create(&ctx, &vault).await?; // TODO: add options to setup entity from creation
                                                    // Rename to create_credential_issuance_key
    entity.create_key(/* Move to Entity? */ Profile::CREDENTIALS_ISSUE.to_string()).await?;

    println!("Office id: {}", entity.identifier().await?);

    entity.create_secure_channel_listener(OFFICE_LISTENER_ADDRESS, TrustEveryonePolicy).await?;

    println!("Enter Bob id: ");
    let bob_id = read_line();
    let bob_id = EntityIdentifier::try_from(bob_id.as_str())?;

    entity.create_credential_issuance_listener(
        OFFICE_ISSUER_ADDRESS.into(),
        credential_type!["TYPE_ID"; "door_id", (Number, "can_open_door")],
        TrustIdentifierPolicy::new(bob_id.clone()),
        // TODO: TrustPolicy doesn't have access to enough data about the requested credential to make a decision
    ).await?;

    Ok(())
}
