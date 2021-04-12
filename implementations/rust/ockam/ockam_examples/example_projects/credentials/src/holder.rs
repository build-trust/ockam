use ockam::{
    async_worker, Context, Credential, CredentialFragment1, CredentialHolder, CredentialVerifier,
    OckamError, OfferIdBytes, PresentationManifest, PublicKeyBytes, Result, Route, Routed, Worker,
};
use ockam_transport_tcp::TcpTransport;

use credentials::{
    example_schema, issuer_on_or_default, CredentialMessage, CredentialRng, DEFAULT_VERIFIER_PORT,
};
use ockam_transport_tcp::{self as tcp, TcpRouter};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use structopt::StructOpt;

struct Holder {
    holder: CredentialHolder,
    issuer: SocketAddr,
    verifier: SocketAddr,
    issuer_pubkey: Option<PublicKeyBytes>,
    frag1: Option<CredentialFragment1>,
    credential: Option<Credential>,
    offer_id: Option<OfferIdBytes>,
}

#[async_worker]
impl Worker for Holder {
    type Message = CredentialMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let issuer = self.issuer;
        let verifier = self.verifier;

        let issuer_pair = TcpTransport::create(&ctx, issuer).await?;
        let verifier_pair = TcpTransport::create(&ctx, verifier).await?;

        // Send a New Credential Connection message
        ctx.send(
            Route::new()
                .append(format!("1#{}", issuer))
                .append("issuer"),
            CredentialMessage::CredentialConnection,
        )
        .await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let route = msg.reply();
        let msg = msg.take();

        match msg {
            CredentialMessage::CredentialIssuer { public_key, proof } => {
                if CredentialVerifier::verify_proof_of_possession(public_key, proof) {
                    self.issuer_pubkey = Some(public_key);

                    ctx.send(route, CredentialMessage::NewCredential).await
                } else {
                    Err(OckamError::InvalidProof.into())
                }
            }
            CredentialMessage::CredentialOffer(offer) => {
                if let Some(issuer_key) = self.issuer_pubkey {
                    let rng = CredentialRng::from_entropy();
                    if let Ok((request, frag1)) =
                        self.holder.accept_credential_offer(&offer, issuer_key, rng)
                    {
                        self.offer_id = Some(offer.id);
                        self.frag1 = Some(frag1);
                        return ctx
                            .send(route, CredentialMessage::CredentialRequest(request))
                            .await;
                    }
                }
                Err(OckamError::InvalidInternalState.into())
            }

            CredentialMessage::CredentialResponse(frag2) => {
                let holder = &self.holder;
                if let Some(frag1) = &self.frag1 {
                    let credential = holder.combine_credential_fragments(frag1.clone(), frag2);
                    self.credential = Some(credential.clone());

                    println!("Credential obtained from Issuer.");

                    let presentation_manifest = PresentationManifest {
                        credential_schema: example_schema(),
                        public_key: self.issuer_pubkey.unwrap(),
                        revealed: vec![1],
                    };

                    let n = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let mut request_id = [0u8; 32];
                    request_id[24..].copy_from_slice(&n.to_be_bytes()[..]);

                    let rng = CredentialRng::from_entropy();
                    if let Ok(presentation) = holder.present_credentials(
                        &[credential],
                        &[presentation_manifest.clone()],
                        request_id,
                        rng,
                    ) {
                        println!("Presenting credentials to Verifier");

                        ctx.send(
                            Route::new()
                                .append(format!("1#{}", self.verifier))
                                .append("verifier"),
                            CredentialMessage::Presentation(presentation, request_id),
                        )
                        .await
                        .unwrap()
                    } else {
                        println!("Failed to create presentation")
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[derive(StructOpt)]
struct Args {
    #[structopt(long, short = "i")]
    issuer: Option<String>,
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Args = Args::from_args();

    // Demo hack to get a reference from Holder to Verifier
    let verifier: SocketAddr = format!("127.0.0.1:{}", DEFAULT_VERIFIER_PORT)
        .parse()
        .unwrap();

    let issuer = issuer_on_or_default(args.issuer);

    let rng = CredentialRng::from_entropy();
    let holder = CredentialHolder::new(rng);

    ctx.start_worker(
        "holder",
        Holder {
            holder,
            issuer,
            verifier,
            issuer_pubkey: None,
            frag1: None,
            credential: None,
            offer_id: None,
        },
    )
    .await
}
