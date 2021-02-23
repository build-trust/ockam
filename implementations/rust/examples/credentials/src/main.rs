use ockam::{
    CredentialAttribute, CredentialAttributeSchema, CredentialAttributeType, CredentialHolder,
    CredentialIssuer, CredentialSchema, CredentialVerifier, PresentationManifest,
};
use std::collections::BTreeMap;

fn main() {
    let schema = CredentialSchema {
        id: "77777777-7777-7777-7777-777777777777".to_string(),
        label: "Truck Management".to_string(),
        description: "A Demoable schema".to_string(),
        attributes: vec![
            CredentialAttributeSchema {
                label: "secretid".to_string(),
                description: "A unique device identifier. ".to_string(),
                attribute_type: CredentialAttributeType::Blob,
            },
            CredentialAttributeSchema {
                label: "device_name".to_string(),
                description: "A friendly name for the device".to_string(),
                attribute_type: CredentialAttributeType::Utf8String,
            },
            CredentialAttributeSchema {
                label: "location".to_string(),
                description: "Where the device is physically located".to_string(),
                attribute_type: CredentialAttributeType::Utf8String,
            },
            CredentialAttributeSchema {
                label: "slot".to_string(),
                description: "Which slot the device is plugged in".to_string(),
                attribute_type: CredentialAttributeType::Number,
            },
            CredentialAttributeSchema {
                label: "interface".to_string(),
                description: "Enumeration for the communication interface".to_string(),
                attribute_type: CredentialAttributeType::Number,
            },
        ],
    };

    // Create a new issuer.
    // CredentialIssuer has a credential signature public key.
    // CredentialIssuer creates a proof of possession
    // so users can verify it.
    // These should be posted such that verifiers and
    // holders can check them
    let issuer = CredentialIssuer::new();
    let pk = issuer.get_public_key();
    let pop = issuer.create_proof_of_possession();

    // A verifying service that receives the public key can check the proof of possession
    // Or a holder can check it prior to accepting a credential offer
    assert!(CredentialVerifier::verify_proof_of_possession(pk, pop));

    let holder = CredentialHolder::new();

    // CredentialIssuer offers holder a credential
    let offer = issuer.create_offer(&schema);

    // CredentialHolder accepts the credential
    // Accepting the offer yields a request to send back to the issuer
    // and a blinding. The blinding is held until the issuer sends
    // a blinded credential. The blinding is used to unblind it and
    // produce the credential.
    // The blinding is a cryptographic commitment that hides the
    // the holder's unique id. The unique id is used to prove
    // that multiple credentials were issued to the same holder.
    let (request, blinding) = holder.accept_credential_offer(&offer, pk).unwrap();

    // Send request to the issuer
    // CredentialIssuer processes the credential request
    // Issuer knows all of the attributes that were not blinded
    // by the holder
    let mut attributes = BTreeMap::new();
    attributes.insert(
        schema.attributes[1].label.clone(),
        CredentialAttribute::String("Robot 1".to_string()),
    );
    attributes.insert(
        schema.attributes[2].label.clone(),
        CredentialAttribute::String("Acme Factory".to_string()),
    );
    attributes.insert(
        schema.attributes[3].label.clone(),
        CredentialAttribute::Numeric(1),
    );
    attributes.insert(
        schema.attributes[4].label.clone(),
        CredentialAttribute::Numeric(1),
    );
    let blind_credential = issuer
        .blind_sign_credential(&request, &schema, &attributes, offer.id)
        .unwrap();

    // Send the blind credential back to the holder
    // who unblinds it. CredentialHolder can then use the credential to prove to a verifier
    let credential = holder.unblind_credential(blind_credential, blinding);

    // CredentialHolder no proves to be a robot in Acme Factory without revealing anything else
    // but first a relying party sends a presentation manifest that indicates what should be
    // revealed and what is allowed to be withheld.
    // Relying party also sends a presentation session number so the
    // holder can't cheat and use a previous proof. This ensures the holder does in fact
    // have a credential.
    let presentation_manifest = PresentationManifest {
        credential_schema: schema, // only accept credentials that match this schema
        public_key: pk,            // only accept credentials issued by this authority
        revealed: vec![2],         // location is required to be revealed
    };
    let request_id = CredentialVerifier::create_proof_request_id();

    // Send manifest and id to holder
    // CredentialHolder creates a presentation
    let presentation = holder
        .present_credentials(&[credential], &[presentation_manifest.clone()], request_id)
        .unwrap();

    // CredentialHolder sends the presentation back to the relying party
    // who can verify it
    assert!(CredentialVerifier::verify_credential_presentations(
        presentation.as_slice(),
        &[presentation_manifest],
        request_id
    )
    .is_ok());
}
