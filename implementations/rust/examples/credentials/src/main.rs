mod util;

use ockam::{
    CredentialAttribute, CredentialHolder, CredentialIssuer, CredentialVerifier,
    PresentationManifest,
};
use std::collections::BTreeMap;
use util::example_schema;

fn main() {
    let schema = example_schema();

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
    // and the first credential fragment. The first fragment is held until the issuer sends
    // the second credential fragment. The fragments are combined to produce the credential.
    // Fragment 1 is a cryptographic commitment that hides the
    // the holder's unique id. The unique id is used to prove
    // that multiple credentials were issued to the same holder.
    let (request, credential_fragment1) = holder.accept_credential_offer(&offer, pk).unwrap();

    // Send request to the issuer

    // CredentialIssuer processes the credential request
    // Issuer knows all of the attributes that were not blinded
    // by the holder
    let mut attributes = BTreeMap::new();
    attributes.insert(
        schema.attributes[1].label.clone(),
        CredentialAttribute::Numeric(1), // TRUE, the device has access
    );

    // Fragment 2 is a partial signature
    let credential_fragment2 = issuer
        .sign_credential_request(&request, &schema, &attributes, offer.id)
        .unwrap();

    // Send the second credential fragment back to the holder
    // who can combine it with the first fragment.
    // CredentialHolder can then use the credential to prove to a verifier
    let credential =
        holder.combine_credential_fragments(credential_fragment1, credential_fragment2);

    // CredentialHolder no proves to be a robot in Acme Factory without revealing anything else
    // but first a relying party sends a presentation manifest that indicates what should be
    // revealed and what is allowed to be withheld.
    // Relying party also sends a presentation session number so the
    // holder can't cheat and use a previous proof. This ensures the holder does in fact
    // have a credential.
    let presentation_manifest = PresentationManifest {
        credential_schema: schema, // only accept credentials that match this schema
        public_key: pk,            // only accept credentials issued by this authority
        revealed: vec![1],         // location is required to be revealed
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
