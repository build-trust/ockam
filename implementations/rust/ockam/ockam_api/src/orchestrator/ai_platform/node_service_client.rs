use crate::nodes::InMemoryNode;
use crate::orchestrator::ai_platform::api::AiPlatformApi;
use crate::orchestrator::ai_platform::responses::{EcrCredentials, Secret, Zone};
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_core::env::get_env_with_default_ignore_error;
use ockam_node::Context;
use once_cell::sync::Lazy;

pub const AI_API_BASE_URL_ENV: &str = "AI_API_BASE_URL";
static AI_API_BASE_URL: Lazy<String> = Lazy::new(|| {
    let v = get_env_with_default_ignore_error(
        AI_API_BASE_URL_ENV,
        "http://localhost:30080".to_string(),
    );
    debug!(url=%v, "using AI API base URL");
    v
});

#[async_trait]
impl AiPlatformApi for InMemoryNode {
    async fn create_zone(
        &self,
        _ctx: &Context,
        customer: &str,
        zone_name: &str,
    ) -> miette::Result<Zone> {
        let url = format!("{}/api/{}/zone", *AI_API_BASE_URL, customer);

        let body = serde_json::json!({
            "zone": zone_name,
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| miette::miette!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "Failed to create zone: HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        let zone = response
            .json::<Zone>()
            .await
            .map_err(|e| miette::miette!("Failed to parse response: {}", e))?;

        Ok(zone)
    }

    async fn list_zones(&self, ctx: &Context, customer: &str) -> miette::Result<Vec<Zone>> {
        let controller = self.create_controller().await?;
        controller
            .list_zones(ctx, customer)
            .await
            .map_err(|e| miette::miette!("Failed to list zones: {}", e))
    }

    async fn delete_zone(
        &self,
        ctx: &Context,
        customer: &str,
        zone_name: &str,
    ) -> miette::Result<()> {
        let url = format!("{}/api/{}/zone/{}", *AI_API_BASE_URL, customer, zone_name);

        let client = reqwest::Client::new();
        let response = client
            .delete(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| miette::miette!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "Failed to delete zone: HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        // Check if the zone was deleted successfully by attempting to list zones
        let max_timeout = std::time::Duration::from_secs(20);
        let start_time = std::time::Instant::now();
        loop {
            let zones = self.list_zones(ctx, customer).await?;
            if zones.iter().all(|zone| zone.zone != zone_name) {
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
        _ctx: &Context,
        customer: &str,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()> {
        let url = format!(
            "{}/api/{}/zone/{}/pods",
            *AI_API_BASE_URL, customer, zone_name
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(zone_config)
            .send()
            .await
            .map_err(|e| miette::miette!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "Failed to deploy zone: HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        Ok(())
    }

    async fn create_secret(
        &self,
        _ctx: &Context,
        customer: &str,
        zone_name: &str,
        secret_name: &str,
        secret_fields: HashMap<String, String>,
    ) -> miette::Result<()> {
        let url = format!(
            "{}/api/{}/zone/{}/secret",
            *AI_API_BASE_URL, customer, zone_name
        );

        let body = serde_json::json!({
            "name": secret_name,
            "fields": secret_fields
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| miette::miette!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "Failed to create secret: HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        Ok(())
    }

    async fn list_secrets(
        &self,
        _ctx: &Context,
        _customer: &str,
        _zone_name: &str,
    ) -> miette::Result<Vec<Secret>> {
        todo!()
    }

    async fn delete_secret(
        &self,
        _ctx: &Context,
        _customer: &str,
        _zone_name: &str,
        _secret_name: &str,
    ) -> miette::Result<()> {
        todo!()
    }

    async fn get_cluster(&self, _ctx: &Context, _zone_name: &str) -> miette::Result<String> {
        Ok(self
            .cli_state
            .get_default_user()
            .await?
            .email
            .domain()?
            .replace('.', "-"))
    }

    async fn provision_ecr(
        &self,
        __ctx: &Context,
        customer: &str,
        image_name: &str,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredentials> {
        let url = format!("{}/api/{}/ecr", *AI_API_BASE_URL, customer);

        let body = serde_json::json!({
            "image_name": image_name,
            "is_public": is_public.unwrap_or(false),
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| miette::miette!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "Failed to provision ECR: HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let ecr_credentials = response
            .json::<EcrCredentials>()
            .await
            .map_err(|e| miette::miette!("Failed to parse response: {}", e))?;

        Ok(ecr_credentials)
    }
}
