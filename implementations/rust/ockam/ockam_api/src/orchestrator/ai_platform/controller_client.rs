use std::collections::BTreeMap;

use crate::orchestrator::ai_platform::api::AiPlatformApi;
use crate::orchestrator::ai_platform::requests::{
    CreateEnrollmentToken, CreateSecret, CreateZone, DeployZone, ListZones, ProvisionEcr,
};
use crate::orchestrator::ai_platform::responses::{
    Cluster, EcrCredentials, Secret, SecretList, Zone, ZoneList,
};
use crate::orchestrator::{ControllerClient, HasSecureClient};
use miette::IntoDiagnostic;
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_node::Context;

#[async_trait]
impl AiPlatformApi for ControllerClient {
    async fn create_zone(&self, ctx: &Context, cluster: &str, name: &str) -> miette::Result<Zone> {
        trace!(%cluster, zone_name = name, "creating zone");
        let req = Request::post("/v0").body(CreateZone::new(name.to_string()));
        self.get_secure_client()
            .ask(ctx, "zones", req)
            .await
            .into_diagnostic()?
            .miette_success("create zone")
    }

    async fn list_zones(&self, ctx: &Context, cluster: &str) -> miette::Result<Vec<String>> {
        trace!(%cluster, "listing zones");
        let req = Request::get("/v0").body(ListZones::new(cluster.to_string()));
        let zones: ZoneList = self
            .get_secure_client()
            .ask(ctx, "zones", req)
            .await
            .into_diagnostic()?
            .miette_success("get zones")?;
        Ok(zones.zones)
    }

    async fn delete_zone(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
    ) -> miette::Result<()> {
        trace!(%cluster, zone_name = zone_name, "deleting zone");
        let req = Request::delete(format!("/v0/zone/{zone_name}"));
        self.get_secure_client()
            .tell(ctx, "zones", req)
            .await
            .into_diagnostic()?
            .miette_success("delete zone")?;

        // Check if the zone was deleted successfully by attempting to list zones
        let max_timeout = std::time::Duration::from_secs(20);
        let start_time = std::time::Instant::now();
        loop {
            let zones = self.list_zones(ctx, cluster).await?;
            if zones.iter().all(|zone| zone != zone_name) {
                break;
            }
            if start_time.elapsed() > max_timeout {
                return Err(miette::miette!("Timeout while waiting for zone deletion"));
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn deploy_zone(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()> {
        trace!(%cluster, zone_name = zone_name, "deploying zone");
        let req =
            Request::post(format!("/v0/zone/{zone_name}/pods")).body(DeployZone::new(zone_config)?);
        self.get_secure_client()
            .tell(ctx, "zones", req)
            .await
            .into_diagnostic()?
            .miette_success("deploy zone")
    }

    async fn create_secret(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
        secret_name: &str,
        secret_fields: &HashMap<String, String>,
    ) -> miette::Result<()> {
        trace!(%cluster, %zone_name, %secret_name, "creating secret");
        let req = Request::post(format!("/v0/zone/{zone_name}"))
            .body(CreateSecret::new(secret_name, secret_fields)?);
        self.get_secure_client()
            .tell(ctx, "secrets", req)
            .await
            .into_diagnostic()?
            .miette_success("create secret")
    }

    async fn list_secrets(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
    ) -> miette::Result<Vec<Secret>> {
        trace!(%cluster, %zone_name, "listing secrets");
        let req = Request::post(format!("/v0/zone/{zone_name}"));
        let secrets: SecretList = self
            .get_secure_client()
            .ask(ctx, "secrets", req)
            .await
            .into_diagnostic()?
            .miette_success("get secrets")?;
        let secrets = secrets
            .secrets
            .into_iter()
            .map(|s| Secret { name: s })
            .collect();
        Ok(secrets)
    }

    async fn delete_secret(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
        secret_name: &str,
    ) -> miette::Result<()> {
        trace!(%cluster, %zone_name, %secret_name, "deleting secret");
        let req = Request::delete(format!("/v0/zone/{zone_name}"));
        self.get_secure_client()
            .tell(ctx, "secrets", req)
            .await
            .into_diagnostic()?
            .miette_success("delete secret")
    }

    async fn create_enrollment_token(
        &self,
        ctx: &Context,
        cluster: &str,
        zone_name: &str,
        attributes: BTreeMap<String, String>,
        relay: Option<String>,
    ) -> miette::Result<String> {
        trace!(%cluster, %zone_name, ?attributes, ?relay, "creating enrollment token");
        let req = Request::post(format!("/v0/zone/{zone_name}/token"))
            .body(CreateEnrollmentToken::new(attributes.clone(), relay)?);
        let token: String = self
            .get_secure_client()
            .ask(ctx, "tokens", req)
            .await
            .into_diagnostic()?
            .miette_success("create enrollment token")?;
        Ok(token)
    }

    async fn get_cluster(&self, ctx: &Context) -> miette::Result<Cluster> {
        trace!("getting cluster");
        let req = Request::get("/v0");
        let cluster: Cluster = self
            .get_secure_client()
            .ask(ctx, "clusters", req)
            .await
            .into_diagnostic()?
            .miette_success("get cluster")?;
        Ok(cluster)
    }

    async fn provision_ecr(
        &self,
        ctx: &Context,
        cluster: &str,
        image_name: &str,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredentials> {
        trace!(%cluster, image_name = image_name, "provisioning ecr");
        let req = Request::post("/v0/ecr")
            .body(ProvisionEcr::new(image_name, is_public.unwrap_or(false)));
        let ecr_creds: EcrCredentials = self
            .get_secure_client()
            .ask(ctx, "clusters", req)
            .await
            .into_diagnostic()?
            .miette_success("provision ecr")?;
        Ok(ecr_creds)
    }
}
