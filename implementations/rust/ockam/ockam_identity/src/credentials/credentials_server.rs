use async_trait::async_trait;

use ockam_core::api::Request;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result, Route};
use ockam_node::api::Client;
use ockam_node::{Context, WorkerBuilder};

use crate::credentials::credentials_server_worker::CredentialsServerWorker;
use crate::credentials::Credentials;
use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::{IdentitySecureChannelLocalInfo, TrustContext};

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
        authorities: &[Identifier],
        credential: CredentialAndPurposeKey,
    ) -> Result<()>;

    /// Present credential to other party, route shall use secure channel
    async fn present_credential(
        &self,
        ctx: &Context,
        route: Route,
        credential: CredentialAndPurposeKey,
    ) -> Result<()>;

    /// Start this service as a worker
    async fn start(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        identifier: Identifier,
        address: Address,
        present_back: bool,
    ) -> Result<()>;
}

/// Implementation of the CredentialsService
pub struct CredentialsServerModule {
    credentials: Arc<Credentials>,
}

#[async_trait]
impl CredentialsServer for CredentialsServerModule {
    /// Present credential to other party, route shall use secure channel. Other party is expected
    /// to present its credential in response, otherwise this call errors.
    async fn present_credential_mutual(
        &self,
        ctx: &Context,
        route: Route,
        authorities: &[Identifier],
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        let path = "actions/present_mutual";
        let client = Client::new(&route, None);
        let (reply, local_info) = client
            .ask_with_local_info(ctx, Request::post(path).body(credential), None)
            .await?;

        let their_id =
            IdentitySecureChannelLocalInfo::find_info_from_list(&local_info)?.their_identity_id();

        let credential_and_purpose_key: CredentialAndPurposeKey = reply.success()?;
        self.credentials
            .credentials_verification()
            .receive_presented_credential(&their_id, authorities, &credential_and_purpose_key)
            .await?;

        Ok(())
    }

    /// Present credential to other party, route shall use secure channel
    async fn present_credential(
        &self,
        ctx: &Context,
        route: Route,
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        let client = Client::new(&route, None);
        client
            .tell(ctx, Request::post("actions/present").body(credential))
            .await?
            .success()
    }

    /// Start worker that will be available to receive others attributes and put them into storage,
    /// after successful verification
    async fn start(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        identifier: Identifier,
        address: Address,
        present_back: bool,
    ) -> Result<()> {
        let worker = CredentialsServerWorker::new(
            self.credentials.clone(),
            trust_context,
            identifier,
            present_back,
        );

        WorkerBuilder::new(worker)
            .with_address(address)
            .start(ctx)
            .await?;

        Ok(())
    }
}

impl CredentialsServerModule {
    /// Create a CredentialsService. It is simply backed by the Credentials interface
    pub fn new(credentials: Arc<Credentials>) -> Self {
        Self { credentials }
    }
}
