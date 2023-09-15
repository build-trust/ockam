use crate::cloud::lease_manager::models::influxdb::Token;
use crate::cloud::ProjectNode;
use ockam_core::api::{Reply, Request};
use ockam_core::{async_trait, Result};
use ockam_node::Context;

#[async_trait]
pub trait InfluxDbTokenLease {
    async fn create_token(&self, ctx: &Context) -> Result<Reply<Token>>;

    async fn get_token(&self, ctx: &Context, token_id: String) -> Result<Reply<Token>>;

    async fn revoke_token(&self, ctx: &Context, token_id: String) -> Result<Reply<()>>;

    async fn list_tokens(&self, ctx: &Context) -> Result<Reply<Vec<Token>>>;
}

#[async_trait]
impl InfluxDbTokenLease for ProjectNode {
    async fn create_token(&self, ctx: &Context) -> Result<Reply<Token>> {
        self.0
            .ask(ctx, "influxdb_token_lease", Request::post("/"))
            .await
    }

    async fn get_token(&self, ctx: &Context, token_id: String) -> Result<Reply<Token>> {
        self.0
            .ask(
                ctx,
                "influxdb_token_lease",
                Request::get(format!("/{token_id}")),
            )
            .await
    }

    async fn revoke_token(&self, ctx: &Context, token_id: String) -> Result<Reply<()>> {
        self.0
            .tell(
                ctx,
                "influxdb_token_lease",
                Request::delete(format!("/{token_id}")),
            )
            .await
    }

    async fn list_tokens(&self, ctx: &Context) -> Result<Reply<Vec<Token>>> {
        self.0
            .ask(ctx, "influxdb_token_lease", Request::get("/"))
            .await
    }
}
