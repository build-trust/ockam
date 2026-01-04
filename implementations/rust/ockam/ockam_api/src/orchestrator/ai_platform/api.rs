use std::collections::BTreeMap;

use crate::orchestrator::ai_platform::responses::{Cluster, EcrCredential, GatewayToken, Secret, Zone};
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;

#[async_trait]
pub trait AiPlatformApi {
    async fn create_zone(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<Zone>;

    async fn list_zones(&self, ctx: &Context, cluster: Option<&str>)
        -> miette::Result<Vec<String>>;

    async fn delete_zone(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<()>;

    async fn deploy_zone(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()>;

    async fn create_secret(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        secret_name: &str,
        secret_fields: &HashMap<String, String>,
    ) -> miette::Result<()>;

    async fn list_secrets(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<Vec<Secret>>;

    async fn delete_secret(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        secret_name: &str,
    ) -> miette::Result<()>;

    async fn create_enrollment_ticket(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        attributes: BTreeMap<String, String>,
        relay: Option<String>,
    ) -> miette::Result<String>;

    async fn get_cluster(&self, ctx: &Context) -> miette::Result<Cluster>;

    async fn provision_ecr(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        image_names: Vec<String>,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredential>;

    /// Create a gateway JWT token for API authentication.
    ///
    /// This token allows zone containers to authenticate with the
    /// autonomy-external-apis-gateway.
    async fn create_gateway_token(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<GatewayToken>;

    /// Create a development token for local development.
    ///
    /// This token is tied to the user's account (not a specific zone) and can be
    /// used for local development before a zone is created. It has a shorter TTL
    /// than production tokens and is intended for development/testing only.
    ///
    /// The token allows access to the gateway but usage is tracked at the user
    /// level rather than zone level.
    async fn create_dev_token(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
    ) -> miette::Result<GatewayToken>;

    /// Create a development enrollment ticket for local development.
    ///
    /// This ticket allows creating a node that can connect to the gateway relay
    /// without requiring a zone to exist. It's used by `autonomy zone dev` to
    /// create a portal to the gateway before a zone is created.
    ///
    /// The ticket grants access to the "gateway" relay which is set up by the
    /// gateway-outlet deployment.
    async fn create_dev_enrollment_ticket(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
    ) -> miette::Result<String>;
}
