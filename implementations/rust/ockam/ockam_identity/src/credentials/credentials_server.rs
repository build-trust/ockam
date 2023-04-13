use crate::credential::Credential;
use crate::credentials::credentials_server_worker::CredentialsServerWorker;
use crate::credentials::Credentials;
use crate::identity::Identity;
use crate::secure_channel::IdentitySecureChannelLocalInfo;
use crate::{IdentityIdentifier, TrustContext};
use async_trait::async_trait;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, AllowAll, Error, Mailboxes, Result, Route};
use ockam_node::api::{request, request_with_local_info};
use ockam_node::{Context, WorkerBuilder};

/// This trait allows an identity to send its credential to another identity
/// located at the end of a secure channel route
#[async_trait]
pub trait CredentialsServer: Send + Sync {
    /// Present credential to other party, route shall use secure channel. Other party is expected
    /// to present its credential in response, otherwise this call errors.
    ///
    async fn present_credential_mutual(
        &self,
        ctx: &Context,
        route: Route,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<()>;

    /// Present credential to other party, route shall use secure channel
    async fn present_credential(
        &self,
        ctx: &Context,
        route: Route,
        credential: Credential,
    ) -> Result<()>;

    /// Start this service as a worker
    async fn start(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        identifier: IdentityIdentifier,
        address: Address,
        present_back: bool,
    ) -> Result<()>;
}

/// Implementation of the CredentialsService
pub struct CredentialsServerModule {
    credentials: Arc<dyn Credentials>,
}

#[async_trait]
impl CredentialsServer for CredentialsServerModule {
    /// Present credential to other party, route shall use secure channel. Other party is expected
    /// to present its credential in response, otherwise this call errors.
    async fn present_credential_mutual(
        &self,
        ctx: &Context,
        route: Route,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<()> {
        let path = "actions/present_mutual";
        let (buf, local_info) = request_with_local_info(
            ctx,
            "credential",
            None,
            route,
            Request::post(path).body(credential),
        )
        .await?;

        let their_id = IdentitySecureChannelLocalInfo::find_info_from_list(&local_info)?
            .their_identity_id()
            .clone();

        let mut dec = Decoder::new(&buf);
        let res: Response = dec.decode()?;
        match res.status() {
            Some(Status::Ok) => {}
            Some(s) => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    format!("credential presentation failed: {}", s),
                ));
            }
            _ => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    "credential presentation failed",
                ));
            }
        }

        let credential: Credential = dec.decode()?;
        self.credentials
            .receive_presented_credential(&their_id, authorities, credential)
            .await?;

        Ok(())
    }

    /// Present credential to other party, route shall use secure channel
    async fn present_credential(
        &self,
        ctx: &Context,
        route: Route,
        credential: Credential,
    ) -> Result<()> {
        let buf = request(
            ctx,
            "credential",
            None,
            route,
            Request::post("actions/present").body(credential),
        )
        .await?;

        let res: Response = minicbor::decode(&buf)?;
        match res.status() {
            Some(Status::Ok) => Ok(()),
            _ => Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "credential presentation failed",
            )),
        }
    }

    /// Start worker that will be available to receive others attributes and put them into storage,
    /// after successful verification
    async fn start(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        identifier: IdentityIdentifier,
        address: Address,
        present_back: bool,
    ) -> Result<()> {
        let worker = CredentialsServerWorker::new(
            self.credentials.clone(),
            trust_context,
            identifier,
            present_back,
        );

        WorkerBuilder::with_mailboxes(
            Mailboxes::main(
                address,
                Arc::new(AllowAll), // We check for Identity secure channel inside the worker
                Arc::new(AllowAll), // FIXME: @ac Allow to respond anywhere using return_route
            ),
            worker,
        )
        .start(ctx)
        .await?;

        Ok(())
    }
}

impl CredentialsServerModule {
    /// Create a CredentialsService. It is simply backed by the Credentials interface
    pub fn new(credentials: Arc<dyn Credentials>) -> Self {
        Self { credentials }
    }
}
