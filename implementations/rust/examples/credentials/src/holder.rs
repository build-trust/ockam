mod util;

use ockam::{
    CredentialFragment2, CredentialHolder, CredentialOffer, CredentialPresentation,
    CredentialRequest, CredentialVerifier, PresentationManifest,
};

use std::time::{SystemTime, UNIX_EPOCH};
use util::example_schema;

fn main() {
    let holder = CredentialHolder::new();

    let (pk, pop, offer) = contact_issuer();
    // A verifying service that receives the public key can check the proof of possession
    // Or a holder can check it prior to accepting a credential offer
    assert!(CredentialVerifier::verify_proof_of_possession(pk, pop));

    // CredentialHolder accepts the credential
    // Accepting the offer yields a request to send back to the issuer
    // and the first credential fragment. The first fragment is held until the issuer sends
    // the second credential fragment. The fragments are combined to produce the credential.
    // Fragment 1 is a cryptographic commitment that hides the
    // the holder's unique id. The unique id is used to prove
    // that multiple credentials were issued to the same holder.
    let (request, credential_fragment1) = holder.accept_credential_offer(&offer, pk).unwrap();

    // Send the request
    let credential_fragment2 = send_credential_request(request);

    let credential =
        holder.combine_credential_fragments(credential_fragment1, credential_fragment2);

    // Connect to a service now
    connect_to_service();

    // Use credential to prove to service
    // The manifest is common to everyone so it can be
    // hardcoded. The manifest only asks for the presenter
    // to prove they have a credential signed by a trusted authority
    // and they can_access is set to true
    let presentation_manifest = PresentationManifest {
        credential_schema: example_schema(), // only accept credentials that match this schema
        public_key: pk,                      // only accept credentials issued by this authority
        revealed: vec![1],                   // location is required to be revealed
    };
    // For the 3-pass protocol, the holder contacts the service requesting something
    // to which the service responds with a request_id to the holder to incorporate
    // into the proof
    // For the 1-pass protocol, the holder uses a timestamp measured in milliseconds
    // which must be within a certain threshold tolerance like 1 minute.
    // Either way, the service should remember which request_ids are valid.
    // If the service generates the request_id, it should only accept a known request_ids.
    // If the service accepts the timestamp, it should only store them for a minute or two
    // If a request_id is used more than once the service will reject the proof.
    // This example demonstrates the 1-pass protocol
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let mut request_id = [0u8; 32];
    request_id[24..].copy_from_slice(&n.to_be_bytes()[..]);
    // Send manifest and id to holder
    // CredentialHolder creates a presentation
    let presentation = holder
        .present_credentials(&[credential], &[presentation_manifest.clone()], request_id)
        .unwrap();
    send_presentation(presentation);
}

fn contact_issuer() -> ([u8; 96], [u8; 48], CredentialOffer) {
    unimplemented!();
}

fn send_credential_request(_request: CredentialRequest) -> CredentialFragment2 {
    unimplemented!();
}

fn connect_to_service() {}

fn send_presentation(_presentation: Vec<CredentialPresentation>) {
    unimplemented!();
}
