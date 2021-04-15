use ockam::{
    async_worker, Context, CredentialAttribute, CredentialIssuer, CredentialSchema, Result, Routed,
    Worker,
};
use ockam_transport_tcp::TcpTransport;

use credentials::CredentialMessage::{CredentialOffer, CredentialResponse};
use credentials::{example_schema, CredentialMessage, DEFAULT_ISSUER_PORT};
use structopt::StructOpt;

pub struct Issuer {
    credential_issuer: CredentialIssuer,
    schema: CredentialSchema,
}

#[async_worker]
impl Worker for Issuer {
    type Message = CredentialMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Issuer listening on {}.", ctx.address());
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        let issuer = &self.credential_issuer;

        let route = msg.return_route();
        let msg = msg.body();

        let public_key = issuer.get_public_key();
        let proof = issuer.create_proof_of_possession();

        let response = match msg {
            CredentialMessage::CredentialConnection => {
                CredentialMessage::CredentialIssuer { public_key, proof }
            }
            CredentialMessage::NewCredential => {
                let rng = rand::thread_rng();
                let offer = issuer.create_offer(&self.schema, rng);
                CredentialOffer(offer)
            }
            CredentialMessage::CredentialRequest(request) => {
                let attributes = [
                    (
                        self.schema.attributes[1].label.clone(),
                        CredentialAttribute::Numeric(1),
                    ), // TRUE, the device has access
                ];

                let credential_fragment2 = issuer
                    .sign_credential_request(&request, &self.schema, &attributes, request.offer_id)
                    .unwrap();

                CredentialResponse(credential_fragment2)
            }
            _ => unimplemented!(),
        };

        ctx.send(route, response).await
    }
}

#[derive(StructOpt)]
struct Args {
    #[structopt(long, short = "k")]
    signing_key: Option<String>,

    #[structopt(long, short)]
    port: Option<usize>,
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let args: Args = Args::from_args();
    let port = args.port.unwrap_or(DEFAULT_ISSUER_PORT);

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.listen(format!("0.0.0.0:{}", port)).await?;

    let credential_issuer = if let Some(signing_key) = args.signing_key {
        CredentialIssuer::with_signing_key_hex(signing_key).unwrap()
    } else {
        let rng = rand::thread_rng();
        CredentialIssuer::new(rng)
    };

    let schema = example_schema();

    ctx.start_worker(
        "issuer",
        Issuer {
            credential_issuer,
            schema,
        },
    )
    .await
}
