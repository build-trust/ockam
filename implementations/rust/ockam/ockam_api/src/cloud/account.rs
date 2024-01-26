use miette::IntoDiagnostic;
use ockam::identity::models::CredentialAndPurposeKey;

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::ControllerClient;
use crate::nodes::InMemoryNode;

const TARGET: &str = "ockam_api::cloud::account";

#[async_trait]
pub trait Accounts {
    async fn get_account_credential(
        &self,
        ctx: &Context,
    ) -> miette::Result<CredentialAndPurposeKey>;

    async fn get_project_admin_credential(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<CredentialAndPurposeKey>;
}

#[async_trait]
impl Accounts for InMemoryNode {
    async fn get_account_credential(
        &self,
        ctx: &Context,
    ) -> miette::Result<CredentialAndPurposeKey> {
        let controller = self.create_controller().await?;
        controller.get_account_credential(ctx).await
    }
    async fn get_project_admin_credential(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<CredentialAndPurposeKey> {
        let controller = self.create_controller().await?;
        controller
            .get_project_admin_credential(ctx, project_id)
            .await
    }
}

impl ControllerClient {
    pub async fn get_account_credential(
        &self,
        ctx: &Context,
    ) -> miette::Result<CredentialAndPurposeKey> {
        trace!(target: TARGET, "getting account credential");
        self.secure_client
            .ask(ctx, "accounts", Request::get("/v0/account"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn get_project_admin_credential(
        &self,
        ctx: &Context,
        project_id: &str,
    ) -> miette::Result<CredentialAndPurposeKey> {
        trace!(target: TARGET, "getting project admin credential");
        self.secure_client
            .ask(
                ctx,
                "accounts",
                Request::get(format!("/v0/project/{}", project_id)),
            )
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
