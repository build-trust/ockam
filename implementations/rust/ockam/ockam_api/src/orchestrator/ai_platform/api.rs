use crate::orchestrator::ai_platform::responses::{EcrCredentials, Secret, Zone};
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;

#[async_trait]
pub trait AiPlatformApi {
    async fn create_zone(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
    ) -> miette::Result<Zone>;

    async fn list_zones(&self, ctx: &Context, customer: &str) -> miette::Result<Vec<Zone>>;

    async fn delete_zone(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
    ) -> miette::Result<()>;

    async fn deploy_zone(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()>;

    async fn create_secret(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
        secret_name: &str,
        secret_fields: HashMap<String, String>,
    ) -> miette::Result<()>;

    async fn list_secrets(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
    ) -> miette::Result<Vec<Secret>>;

    async fn delete_secret(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
        secret_name: &str,
    ) -> miette::Result<()>;

    async fn get_cluster(&self, ctx: &Context, zone_name: &str) -> miette::Result<String>;

    async fn provision_ecr(
        &self,
        ctx: &Context,
        customer: &str,
        image_name: &str,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredentials>;
}
