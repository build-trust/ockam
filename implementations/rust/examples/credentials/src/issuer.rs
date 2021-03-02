mod util;

use ockam::{CredentialAttribute, CredentialIssuer};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;
use util::{example_schema, CredentialMessage, Listener, Stream};

#[derive(StructOpt)]
struct Args {
    /// HEX encoded issuer signing key
    #[structopt(long)]
    secret_key: Option<String>,
    #[structopt(long)]
    port: Option<usize>,
    #[structopt(long, parse(from_os_str))]
    socket: Option<PathBuf>,
}

fn main() {
    let args = Args::from_args();

    // Create a new issuer.
    // CredentialIssuer has a credential signature public key.
    // CredentialIssuer creates a proof of possession
    // so users can verify it.
    // These should be posted such that verifiers and
    // holders can check them
    let issuer = if args.secret_key.is_none() {
        CredentialIssuer::new()
    } else {
        let sk = <[u8; 32]>::try_from(
            hex::decode(args.secret_key.as_ref().unwrap())
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        CredentialIssuer::with_signing_key(sk)
    };
    let pk = issuer.get_public_key();
    let pop = issuer.create_proof_of_possession();
    let schema = example_schema();
    let mut pending_offers = BTreeSet::new();

    //TODO: publish public key and proof of possession for verifiers
    let listener = Listener::bind(args.socket, args.port).unwrap();

    loop {
        let mut stream = listener.accept();

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
                CredentialMessage::CredentialConnection => {
                    serde_bare::to_writer(
                        &mut stream,
                        &CredentialMessage::CredentialIssuer {
                            public_key: pk,
                            proof: pop,
                        },
                    )
                    .unwrap();
                    stream.flush().unwrap();
                }
                CredentialMessage::NewCredential => {
                    // CredentialIssuer offers holder a credential
                    let offer = issuer.create_offer(&schema);
                    pending_offers.insert(offer.id);
                    serde_bare::to_writer(&mut stream, &CredentialMessage::CredentialOffer(offer))
                        .unwrap();
                    stream.flush().unwrap();
                }
                CredentialMessage::CredentialRequest(request) => {
                    if !pending_offers.contains(&request.offer_id) {
                        eprintln!("Unexpected offer id: {:?}", request.offer_id);
                        serde_bare::to_writer(
                            &mut stream,
                            &CredentialMessage::InvalidCredentialRequest,
                        )
                        .unwrap();
                        stream.flush().unwrap();
                        continue;
                    }
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
                        .sign_credential_request(&request, &schema, &attributes, request.offer_id)
                        .unwrap();
                    serde_bare::to_writer(
                        &mut stream,
                        &CredentialMessage::CredentialResponse(credential_fragment2),
                    )
                    .unwrap();
                    stream.flush().unwrap();
                    pending_offers.remove(&request.offer_id);
                }
                _ => {
                    eprintln!("Unhandled message: {:?}", m);
                }
            }
        }
    }
}
