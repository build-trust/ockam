use lib::{read_line, DoorCredentialPresentation, DoorOpenRequest, DoorOpenRequestId};
use ockam::{
    check_message_origin, Context, CredentialAttribute, CredentialVerifier, Entity, LocalInfo,
    Message, NoOpTrustPolicy, ProfileIdentifier, ProfileIdentity, Result, Routed, TcpTransport,
};
use rand::thread_rng;
use std::convert::TryInto;

const DOOR_TCP_ADDRESS: &str = "127.0.0.1:5333";
const DOOR_LISTENER_ADDRESS: &str = "door_listener";

fn get_profile_id<T: Message>(msg: &Routed<T>) -> Result<ProfileIdentifier> {
    let local_msg = msg.local_message();
    let local_info = LocalInfo::decode(local_msg.local_info())?;

    Ok(local_info.their_profile_id().clone())
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Door (Verifier) that trusts the Office, and will checks Bob's (Holder) credentials.
    let mut entity = Entity::create(&ctx).await?;

    println!(
        "Door id: {}",
        entity.identifier()?.to_string_representation()
    );

    println!("Enter Office pubkey: ");
    let office_pubkey = read_line();
    let office_pubkey = hex::decode(office_pubkey).unwrap();
    let office_pubkey: [u8; 96] = office_pubkey.try_into().unwrap();

    entity
        .create_secure_channel_listener(DOOR_LISTENER_ADDRESS, NoOpTrustPolicy {})
        .await?;

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(DOOR_TCP_ADDRESS).await?;

    let request = ctx.receive_timeout::<DoorOpenRequest>(120).await?.take();
    let bob_id = get_profile_id(&request)?;
    let route = request.return_route();

    let unique_opening_instance = entity.create_proof_request_id(thread_rng())?;
    ctx.send(route, DoorOpenRequestId(unique_opening_instance))
        .await?;

    let presentation = ctx
        .receive_timeout::<DoorCredentialPresentation>(120)
        .await?
        .take();
    check_message_origin(&presentation, &bob_id)?;

    // Door (Verifier) verifies that Bob's Presentation is valid (trusted by Office)
    let credential_is_valid = entity.verify_credential_presentations(
        presentation.body().0.as_slice(),
        &[presentation_manifest(office_pubkey)],
        unique_opening_instance,
    )?;

    // The door credential is valid.
    assert!(credential_is_valid);

    let signing_attributes = [
        (
            "door_id".to_string(),
            CredentialAttribute::String("f4a8-90ff-742d-11ae".into()),
        ),
        ("can_open_door".to_string(), CredentialAttribute::Numeric(1)),
    ];

    // Now the actual underlying control attribute can be checked.
    let control = signing_attributes[1].clone();
    let open_door = match control.1 {
        CredentialAttribute::Numeric(i) => i > 0,
        _ => false,
    };

    // The door opens!
    assert!(open_door);

    println!("Door is opened!");

    Ok(())
}
