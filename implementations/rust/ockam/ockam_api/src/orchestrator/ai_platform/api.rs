use crate::orchestrator::ai_platform::models::{EcrCredentials, Zone};
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;

#[async_trait]
pub trait AiPlatformApi {
    async fn create_zone(&self, customer: &str, zone_name: &str) -> miette::Result<Zone>;
    async fn list_zones(&self, customer: &str) -> miette::Result<Vec<Zone>>;
    async fn delete_zone(&self, customer: &str, zone_name: &str) -> miette::Result<()>;
    async fn deploy_zone(
        &self,
        customer: &str,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()>;

    async fn create_secret(
        &self,
        customer: &str,
        zone_name: &str,
        secret_name: &str,
        secret_fields: HashMap<String, String>,
    ) -> miette::Result<()>;
    async fn list_secrets(&self, customer: &str, zone_name: &str) -> miette::Result<Vec<String>>;
    async fn delete_secret(
        &self,
        customer: &str,
        zone_name: &str,
        secret_name: &str,
    ) -> miette::Result<()>;

    async fn provision_ecr(
        &self,
        customer: &str,
        image_name: &str,
        region: Option<&str>,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredentials>;
}
