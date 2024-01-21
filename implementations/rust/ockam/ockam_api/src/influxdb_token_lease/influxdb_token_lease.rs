use crate::cloud::lease_manager::models::influxdb::Token;
use crate::cloud::ProjectNodeClient;
use miette::IntoDiagnostic;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
pub trait InfluxDbTokenLease {
    async fn create_token(&self, ctx: &Context) -> miette::Result<Token>;

    async fn get_token(&self, ctx: &Context, token_id: String) -> miette::Result<Token>;

    async fn revoke_token(&self, ctx: &Context, token_id: String) -> miette::Result<()>;

    async fn list_tokens(&self, ctx: &Context) -> miette::Result<Vec<Token>>;
}

#[async_trait]
impl InfluxDbTokenLease for ProjectNodeClient {
    async fn create_token(&self, ctx: &Context) -> miette::Result<Token> {
        self.secure_client
            .ask(ctx, "influxdb_token_lease", Request::post("/"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn get_token(&self, ctx: &Context, token_id: String) -> miette::Result<Token> {
        self.secure_client
            .ask(
                ctx,
                "influxdb_token_lease",
                Request::get(format!("/{token_id}")),
            )
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn revoke_token(&self, ctx: &Context, token_id: String) -> miette::Result<()> {
        self.secure_client
            .tell(
                ctx,
                "influxdb_token_lease",
                Request::delete(format!("/{token_id}")),
            )
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_tokens(&self, ctx: &Context) -> miette::Result<Vec<Token>> {
        self.secure_client
            .ask(ctx, "influxdb_token_lease", Request::get("/"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
