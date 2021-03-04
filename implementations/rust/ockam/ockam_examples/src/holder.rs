mod util;

use ockam::{CredentialHolder, CredentialVerifier, PresentationManifest};

use std::{
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use structopt::StructOpt;
use util::{example_schema, CredentialMessage, Stream};

#[derive(StructOpt)]
struct Args {
    #[structopt(long)]
    issuer_port: Option<usize>,
    #[structopt(long, parse(from_os_str))]
    issuer_socket: Option<PathBuf>,
    #[structopt(long)]
    verifier_port: Option<usize>,
    #[structopt(long, parse(from_os_str))]
    verifier_socket: Option<PathBuf>,
}

fn main() {
    let args = Args::from_args();

    let holder = CredentialHolder::new();

    let mut stream = Stream::connect(args.issuer_socket, args.issuer_port).unwrap();

    serde_bare::to_writer(&mut stream, &CredentialMessage::CredentialConnection)
        .expect("Unable to contact the issuer");
    stream.flush().unwrap();

    let reader = stream.try_clone().unwrap();
    let msg = serde_bare::from_reader::<Stream, CredentialMessage>(reader)
        .expect("Unable to read message from issuer");

    let pk;
    if let CredentialMessage::CredentialIssuer { public_key, proof } = msg {
        // A verifying service that receives the public key can check the proof of possession
        // Or a holder can check it prior to accepting a credential offer
        assert!(CredentialVerifier::verify_proof_of_possession(
            public_key, proof
        ));
        pk = public_key;
    } else {
        eprintln!("Unexpected message returned from Issuer");
        return;
    }

    // Ask for a new credential
    serde_bare::to_writer(&mut stream, &CredentialMessage::NewCredential)
        .expect("Unable to ask for new credential from issuer");
    stream.flush().unwrap();

    let offer;
    let reader = stream.try_clone().unwrap();
    let msg = serde_bare::from_reader::<Stream, CredentialMessage>(reader)
        .expect("Unable to read message from issuer");

    if let CredentialMessage::CredentialOffer(o) = msg {
        offer = o;
    } else {
        eprintln!("Unexpected message returned from Issuer");
        return;
    }

    // CredentialHolder accepts the credential
    // Accepting the offer yields a request to send back to the issuer
    // and the first credential fragment. The first fragment is held until the issuer sends
    // the second credential fragment. The fragments are combined to produce the credential.
    // Fragment 1 is a cryptographic commitment that hides the
    // the holder's unique id. The unique id is used to prove
    // that multiple credentials were issued to the same holder.
    let (request, credential_fragment1) = holder.accept_credential_offer(&offer, pk).unwrap();

    // Send the request
    serde_bare::to_writer(&mut stream, &CredentialMessage::CredentialRequest(request))
        .expect("Unable to send credential request");
    stream.flush().unwrap();

    let reader = stream.try_clone().unwrap();
    let msg = serde_bare::from_reader::<Stream, CredentialMessage>(reader)
        .expect("Unable to read message from issuer");

    let credential;
    if let CredentialMessage::CredentialResponse(credential_fragment2) = msg {
        credential =
            holder.combine_credential_fragments(credential_fragment1, credential_fragment2);
    } else {
        eprintln!("Unexpected message returned from Issuer");
        return;
    }

    stream
        .shutdown()
        .expect("Unable to close connection to issuer");

    // Connect to a service now
    let mut stream = Stream::connect(args.verifier_socket, args.verifier_port).unwrap();

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
    serde_bare::to_writer(&mut stream, &CredentialMessage::Presentation(presentation))
        .expect("Unable to send presentation");
    stream.flush().unwrap();
    //TODO: what to do now?
}
