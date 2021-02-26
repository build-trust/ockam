mod util;

use ockam::{
    CredentialAttribute, CredentialFragment2, CredentialIssuer, CredentialOffer, CredentialRequest,
};
use std::collections::BTreeMap;
use util::example_schema;

fn main() {
    // Create a new issuer.
    // CredentialIssuer has a credential signature public key.
    // CredentialIssuer creates a proof of possession
    // so users can verify it.
    // These should be posted such that verifiers and
    // holders can check them
    let issuer = CredentialIssuer::new();
    let pk = issuer.get_public_key();
    let pop = issuer.create_proof_of_possession();

    //TODO: publish public key and proof of possession for verifiers

    // Wait for a holder to connect
    listen_for_holder(pk, pop);

    // CredentialIssuer offers holder a credential
    let schema = example_schema();
    let offer = issuer.create_offer(&schema);

    // Send offer to holder
    let request = send_holder_offer(&offer);

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
    send_credential_fragment2(credential_fragment2);
}

fn listen_for_holder(_pk: [u8; 96], _pop: [u8; 48]) {}

fn send_holder_offer(_offer: &CredentialOffer) -> CredentialRequest {
    unimplemented!();
}

fn send_credential_fragment2(_fragment2: CredentialFragment2) {}
