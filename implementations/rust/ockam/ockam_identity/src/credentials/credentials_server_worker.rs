use minicbor::Decoder;
use tracing::{debug, error, info, trace, warn};

use ockam_core::api::{RequestHeader, Response};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::{string::ToString, sync::Arc, vec::Vec};
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;

use crate::credentials::Credentials;
use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::{IdentitySecureChannelLocalInfo, TrustContext};

const TARGET: &str = "ockam::credential_exchange_worker::service";

/// Worker responsible for receiving and verifying other party's credential
pub struct CredentialsServerWorker {
    credentials: Arc<Credentials>,
    trust_context: TrustContext,
    identifier: Identifier,
    present_back: bool,
}

impl CredentialsServerWorker {
    pub fn new(
        credentials: Arc<Credentials>,
        trust_context: TrustContext,
        identifier: Identifier,
        present_back: bool,
    ) -> Self {
        Self {
            credentials,
            trust_context,
            identifier,
            present_back,
        }
    }
}

impl CredentialsServerWorker {
    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &RequestHeader,
        sender: Identifier,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        trace! {
            target: TARGET,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use ockam_core::api::Method::*;
        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => {
                return Ok(Response::bad_request(req, "Invalid method").to_vec()?);
            }
        };

        let r = match (method, path_segments.as_slice()) {
            (Post, ["actions", "present"]) => {
                debug!(
                    "Received one-way credential presentation request from {}",
                    sender
                );
                let credential_and_purpose_key: CredentialAndPurposeKey = dec.decode()?;

                let res = self
                    .credentials
                    .credentials_verification()
                    .receive_presented_credential(
                        &sender,
                        self.trust_context.authorities().await?.as_slice(),
                        &credential_and_purpose_key,
                    )
                    .await;

                match res {
                    Ok(()) => {
                        debug!("One-way credential presentation request processed successfully with {}", sender);
                        Response::ok(req).to_vec()?
                    }
                    Err(err) => {
                        debug!(
                            "One-way credential presentation request processing error: {} for {}",
                            err, sender
                        );
                        Response::bad_request(req, &err.to_string()).to_vec()?
                    }
                }
            }
            (Post, ["actions", "present_mutual"]) => {
                debug!(
                    "Received mutual credential presentation request from {}",
                    sender
                );
                let credential_and_purpose_key: CredentialAndPurposeKey = dec.decode()?;

                // FIXME info!("presented credential {}", credential);
                let res = self
                    .credentials
                    .credentials_verification()
                    .receive_presented_credential(
                        &sender,
                        self.trust_context.authorities().await?.as_slice(),
                        &credential_and_purpose_key,
                    )
                    .await;

                if let Err(err) = res {
                    debug!(
                        "Mutual credential presentation request processing error: {} from {}",
                        err, sender
                    );
                    Response::bad_request(req, &err.to_string()).to_vec()?
                } else {
                    debug!(
                        "Mutual credential presentation request processed successfully with {}",
                        sender
                    );
                    let credential = self
                        .trust_context
                        .authority()?
                        .credential(ctx, &self.identifier)
                        .await;
                    match credential.as_ref() {
                        Ok(p) if self.present_back => {
                            info!("Mutual credential presentation request processed successfully with {}. Responding with own credential...", sender);
                            Response::ok(req).body(p).to_vec()?
                        }
                        _ => {
                            info!("Mutual credential presentation request processed successfully with {}. No credential to respond!", sender);
                            Response::ok(req).to_vec()?
                        }
                    }
                }
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req, &format!("Invalid endpoint: {}", path)).to_vec()?
            }
        };
        Ok(r)
    }
}

#[async_trait]
impl Worker for CredentialsServerWorker {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut dec = Decoder::new(msg.as_body());
        let req: RequestHeader = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let sender =
            IdentitySecureChannelLocalInfo::find_info(msg.local_message())?.their_identity_id();

        let r = match self.handle_request(ctx, &req, sender, &mut dec).await {
            Ok(r) => r,
            // If an error occurs, send a response with the error code so the listener can
            // fail fast instead of failing silently here and force the listener to timeout.
            Err(err) => {
                error!(?err, "Failed to handle message");
                Response::internal_error(&req, &err.to_string()).to_vec()?
            }
        };
        ctx.send(msg.return_route(), r).await
    }
}
