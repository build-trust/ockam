use ockam::{
    async_worker, Context, CredentialVerifier, OckamError, PresentationManifest, PublicKeyBytes,
    Result, Route, Routed, Worker,
};

use credentials::{example_schema, issuer_on_or_default, CredentialMessage, DEFAULT_VERIFIER_PORT};
use ockam_transport_tcp::{TcpTransport, TCP};
use structopt::StructOpt;

struct Verifier {
    issuer: String,
    issuer_pubkey: Option<PublicKeyBytes>,
}

#[async_worker]
impl Worker for Verifier {
    type Message = CredentialMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let issuer = &self.issuer;

        println!("Verifier starting. Discovering Issuer");

        // Send a New Credential Connection message
        ctx.send(
            Route::new().append_t(TCP, issuer).append("issuer"),
            CredentialMessage::CredentialConnection,
        )
        .await
    }

    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg = msg.body();

        match msg {
            CredentialMessage::CredentialIssuer { public_key, proof } => {
                if CredentialVerifier::verify_proof_of_possession(public_key, proof) {
                    self.issuer_pubkey = Some(public_key);
                    println!("Discovered Issuer Pubkey: {}", hex::encode(public_key));
                    Ok(())
                } else {
                    Err(OckamError::InvalidProof.into())
                }
            }
            CredentialMessage::Presentation(_presentation, proof_request_id) => {
                println!("Holder presented credentials.");

                let presentation_manifest = PresentationManifest {
                    credential_schema: example_schema(),
                    public_key: self.issuer_pubkey.unwrap(),
                    revealed: vec![1],
                };

                if let Ok(_) = CredentialVerifier::verify_credential_presentations(
                    _presentation.as_slice(),
                    &[presentation_manifest.clone()],
                    proof_request_id,
                ) {
                    println!("Credential is valid!");
                } else {
                    println!("Invalid credential.");
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

    #[structopt(long, short)]
    port: Option<usize>,
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Args = Args::from_args();
    let port = args.port.unwrap_or(DEFAULT_VERIFIER_PORT);

    let local_tcp = format!("0.0.0.0:{}", port);

    let issuer = issuer_on_or_default(args.issuer);

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(local_tcp).await?;
    tcp.connect(&issuer).await?;

    ctx.start_worker(
        "verifier",
        Verifier {
            issuer,
            issuer_pubkey: None,
        },
    )
    .await
}
