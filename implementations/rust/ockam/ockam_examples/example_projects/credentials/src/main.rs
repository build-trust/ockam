use ockam::{Context, Entity, Result};
use rand::thread_rng;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // The Office is an Issuer of credentials. In this case, permission to open doors.
    let mut office = Entity::create(&ctx)?;

    // Bob is a Holder of a credential, allowing him to open doors.
    let mut bob = Entity::create(&ctx)?;

    // A schema that represents the office doors.
    let door_schema = door_schema();

    // The Issuer (Office) creates an Credential Request Offer (ability to open the door)
    let door_offer = office.create_offer(&door_schema, thread_rng())?;

    // Public key identifying the Issuer (Office)
    let office_pubkey = office.get_issuer_public_key()?;

    // Bob accepts the credential request offer, and creates a credential request, along with the first fragment.
    let (door_request, frag1) =
        bob.accept_credential_offer(&door_offer, office_pubkey, thread_rng())?;

    // Ask the Issuer to sign the Credential Request. A successful request results in a second fragment.
    let signing_attributes = [
        (
            "door_id".into(),
            CredentialAttribute::String("f4a8-90ff-742d-11ae".into()),
        ),
        ("can_open_door".into(), CredentialAttribute::Numeric(1)),
    ];

    // Office signs the credentials.
    let frag2 = office.sign_credential_request(
        &door_request,
        &door_schema,
        &(signing_attributes.clone()),
        door_offer.id,
    )?;

    // Bob can now combine both fragments to form a Credential.
    let bob_credential = bob.combine_credential_fragments(frag1, frag2)?;

    // Create a Door (Verifier) that trusts the Office, and will checks Bob's (Holder) credentials.
    let mut door = Entity::create(&ctx).await?;
    let unique_opening_instance = door.create_proof_request_id(thread_rng())?;

    // Bob (Holder) attempts to open the Door (Verifier). He creates a Presentation Manifest.
    let manifest = PresentationManifest {
        credential_schema: door_schema.clone(),
        public_key: office_pubkey,
        revealed: vec![1], // can_open_door
    };

    // Bob creates a Presentation from the manifest, his credentials, and this unique challenge instance.
    let bob_door_swipe = bob.present_credentials(
        &[bob_credential],
        &[manifest.clone()],
        unique_opening_instance,
        thread_rng(),
    )?;
    assert!(!bob_door_swipe.is_empty());

    // Door (Verifier) verifies that Bob's Presentation is valid (trusted by Office)
    let credential_is_valid = door.verify_credential_presentations(
        bob_door_swipe.as_slice(),
        &[manifest],
        unique_opening_instance,
    )?;

    // The door credential is valid.
    assert!(credential_is_valid);

    // Now the actual underlying control attribute can be checked.
    let control = signing_attributes[1].clone();
    let open_door = match control.1 {
        CredentialAttribute::Numeric(i) => i > 0,
        _ => false,
    };

    // The door opens!
    assert!(open_door);
    ctx.stop().await
}

fn door_schema() -> CredentialSchema {
    CredentialSchema {
        id: "Office".to_string(),
        label: String::new(),
        description: String::new(),
        attributes: vec![
            CredentialAttributeSchema {
                label: "door_id".to_string(),
                description: String::new(),
                unknown: false,
                attribute_type: CredentialAttributeType::Utf8String,
            },
            CredentialAttributeSchema {
                label: "can_open_door".to_string(),
                description: "Is allowed to open the door identified by door_device_id".to_string(),
                unknown: false,
                attribute_type: CredentialAttributeType::Number,
            },
            CredentialAttributeSchema {
                label: "secret_id".to_string(),
                description: "secret id".to_string(),
                unknown: true,
                attribute_type: CredentialAttributeType::Number,
            },
        ],
    }
}
