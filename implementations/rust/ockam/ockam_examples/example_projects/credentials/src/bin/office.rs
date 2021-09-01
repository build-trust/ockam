use lib::{read_line, OFFICE_ISSUER_ADDRESS, OFFICE_LISTENER_ADDRESS, OFFICE_TCP_ADDRESS};
use ockam::{
    credential_type, Context, CredentialProtocol, Entity, EntityIdentifier, Identity, Profile,
    Result, SecureChannels, TcpTransport, TrustEveryonePolicy, TrustIdentifierPolicy, Vault,
};
use std::convert::TryFrom;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(OFFICE_TCP_ADDRESS).await?;

    // The Office is an Issuer of credentials. In this case, permission to open doors.
    let vault = Vault::create(&ctx)?;
    let mut entity = Entity::create(&ctx, &vault)?; // TODO: add options to setup entity from creation
                                                    // Rename to create_credential_issuance_key
    entity.create_key(/* Move to Entity? */ Profile::CREDENTIALS_ISSUE)?;

    println!("Office id: {}", entity.identifier()?);

    entity.create_secure_channel_listener(OFFICE_LISTENER_ADDRESS, TrustEveryonePolicy)?;

    println!("Enter Bob id: ");
    let bob_id = read_line();
    let bob_id = EntityIdentifier::try_from(bob_id.as_str())?;

    entity.create_credential_issuance_listener(
        OFFICE_ISSUER_ADDRESS,
        credential_type!["TYPE_ID"; "door_id", (Number, "can_open_door")],
        TrustIdentifierPolicy::new(bob_id.clone()),
        // TODO: TrustPolicy doesn't have access to enough data about the requested credential to make a decision
    )?;

    Ok(())
}
