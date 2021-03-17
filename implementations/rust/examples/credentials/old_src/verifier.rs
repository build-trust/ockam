mod util;

use ockam::{CredentialVerifier, PresentationManifest};
use std::{
    convert::TryFrom,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use structopt::StructOpt;
use util::{example_schema, CredentialMessage, Listener, Stream};

#[derive(StructOpt)]
struct Args {
    /// HEX encoded issuer signing key
    #[structopt(long)]
    issuer_pk: String,
    #[structopt(long)]
    port: Option<usize>,
    #[structopt(long, parse(from_os_str))]
    socket: Option<PathBuf>,
}

fn main() {
    let args = Args::from_args();

    // TODO: retrieve public key from published location
    // Placeholder for now
    let pk = <[u8; 96]>::try_from(hex::decode(args.issuer_pk).unwrap()).unwrap();

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

    let listener = Listener::bind(args.socket, args.port).unwrap();

    loop {
        let stream = listener.accept();

        loop {
            let reader = stream.try_clone().unwrap();
            let res = serde_bare::from_reader::<Stream, CredentialMessage>(reader);
            if res.is_err() {
                match res.unwrap_err() {
                    serde_bare::Error::Io(e) => match e.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            eprintln!("Client closed connection");
                            break;
                        }
                        _ => {
                            eprintln!("Unknown message type");
                            continue;
                        }
                    },
                    _ => {
                        eprintln!("Unknown message type");
                        continue;
                    }
                }
            }
            let m = res.unwrap();

            match m {
                CredentialMessage::Presentation(presentations) => {
                    // 1-pass protocol checks the timestamp if its been seen before and fresh enough
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let timestamp = u64::from_be_bytes(
                        <[u8; 8]>::try_from(&presentations[0].presentation_id[24..]).unwrap(),
                    );
                    let valid_timestamp = match now.checked_sub(Duration::from_millis(timestamp)) {
                        None => false,
                        Some(d) => d.as_secs() < 60,
                    };

                    if valid_timestamp {
                        // TODO: save for another minute to prevent replay

                        // CredentialHolder sends the presentation to the service
                        // who can verify it
                        assert!(CredentialVerifier::verify_credential_presentations(
                            presentations.as_slice(),
                            &[presentation_manifest.clone()],
                            presentations[0].presentation_id
                        )
                        .is_ok());
                        // TODO: Send message back for success or failure
                    }
                }
                _ => {
                    eprintln!("Unhandled message: {:?}", m);
                }
            }
        }
    }
}
