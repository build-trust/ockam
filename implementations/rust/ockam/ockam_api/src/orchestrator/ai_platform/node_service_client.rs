use crate::nodes::InMemoryNode;
use crate::orchestrator::ai_platform::api::AiPlatformApi;
use crate::orchestrator::ai_platform::responses::{
    Cluster, EcrCredential, GatewayToken, Secret, SecretNameList, Ticket, Zone, ZoneNameList,
};
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_core::env::get_env_with_default_ignore_error;
use ockam_node::Context;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

pub const AI_API_BASE_URL_ENV: &str = "AI_API_BASE_URL";
pub static AI_API_BASE_URL: Lazy<String> = Lazy::new(|| {
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
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<Zone> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to create zone {zone_name} in cluster {cluster}");
        let url = format!("{}/api/{}/zone", *AI_API_BASE_URL, cluster);

        let body = serde_json::json!({
            "zone": zone_name,
        });

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        response
            .json::<Zone>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())
    }

    async fn list_zones(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
    ) -> miette::Result<Vec<String>> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to list zones in cluster {cluster}");
        let url = format!("{}/api/{}/zone", *AI_API_BASE_URL, cluster);

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        let res = response
            .json::<ZoneNameList>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())?;

        Ok(res.zones)
    }

    async fn delete_zone(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<()> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to delete zone {zone_name} in cluster {cluster}");
        let url = format!("{}/api/{}/zone/{}", *AI_API_BASE_URL, &cluster, zone_name);

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .delete(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        // Check if the zone was deleted successfully by attempting to list zones
        let max_timeout = std::time::Duration::from_secs(20);
        let start_time = std::time::Instant::now();
        loop {
            let zones = self.list_zones(ctx, Some(&cluster)).await?;
            if zones.iter().all(|zone| zone != zone_name) {
                break;
            }
            if start_time.elapsed() > max_timeout {
                return Err(miette::miette!("Timeout while waiting for zone deletion")
                    .wrap_err(base_error()));
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }

    async fn deploy_zone(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        zone_config: &serde_json::Value,
    ) -> miette::Result<()> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to deploy zone {zone_name} in cluster {cluster}");
        let url = format!(
            "{}/api/{}/zone/{}/pods",
            *AI_API_BASE_URL, cluster, zone_name
        );

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(zone_config)
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        Ok(())
    }

    async fn create_secret(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        secret_name: &str,
        secret_fields: &HashMap<String, String>,
    ) -> miette::Result<()> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || {
            miette!(
                "Failed to create secret {secret_name} for cluster {cluster} and zone {zone_name}"
            )
        };
        let url = format!(
            "{}/api/{}/zone/{}/secret",
            *AI_API_BASE_URL, cluster, zone_name
        );

        let body = serde_json::json!({
            "name": secret_name,
            "fields": secret_fields
        });

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        Ok(())
    }

    async fn list_secrets(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<Vec<Secret>> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error =
            || miette!("Failed to list secrets for cluster {cluster} and zone {zone_name}");
        let url = format!(
            "{}/api/{}/zone/{}/secret",
            *AI_API_BASE_URL, cluster, zone_name
        );

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        let secrets = response
            .json::<SecretNameList>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse secrets")
            .wrap_err(base_error())?;
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
        cluster: Option<&str>,
        zone_name: &str,
        secret_name: &str,
    ) -> miette::Result<()> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || {
            miette!(
                "Failed to delete secret {secret_name} for cluster {cluster} and zone {zone_name}"
            )
        };
        let url = format!(
            "{}/api/{}/zone/{}/secret/{}",
            *AI_API_BASE_URL, cluster, zone_name, secret_name
        );

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .delete(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        Ok(())
    }

    async fn create_enrollment_ticket(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
        attributes: BTreeMap<String, String>,
        relay: Option<String>,
    ) -> miette::Result<String> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || {
            miette!("Failed to create enrollment token for cluster {cluster} and zone {zone_name}")
        };
        let url = format!(
            "{}/api/{}/zone/{}/token",
            *AI_API_BASE_URL, cluster, zone_name
        );

        let mut body = serde_json::json!({
            "attributes": attributes
            .into_iter()
            .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
            .collect::<Vec<_>>(),
        });

        if let Some(relay_value) = relay {
            body["relay"] = serde_json::Value::String(relay_value);
        }

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        let ticket = response
            .json::<Ticket>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())?;

        Ok(ticket.ticket)
    }

    async fn get_cluster(&self, _ctx: &Context) -> miette::Result<Cluster> {
        unimplemented!("get_cluster is only available through the controller client")
    }

    async fn provision_ecr(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        image_names: Vec<String>,
        is_public: Option<bool>,
    ) -> miette::Result<EcrCredential> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to provision ECR for images in cluster {cluster}");
        let url = format!("{}/api/{}/ecr", *AI_API_BASE_URL, cluster);
        let is_public = is_public.unwrap_or(false);
        // TODO: remove once latest provisioner gets deployed
        let region = if is_public { "us-east-1" } else { "us-west-2" };

        let body = serde_json::json!({
            "image_names": image_names,
            "is_public": is_public,
            "region": region,
        });

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        response
            .json::<EcrCredential>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())
    }

    async fn create_gateway_token(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
        zone_name: &str,
    ) -> miette::Result<GatewayToken> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error =
            || miette!("Failed to create gateway token for cluster {cluster} and zone {zone_name}");
        let url = format!(
            "{}/api/{}/zone/{}/gateway-token",
            *AI_API_BASE_URL, cluster, zone_name
        );

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        response
            .json::<GatewayToken>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())
    }

    async fn create_dev_token(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
    ) -> miette::Result<GatewayToken> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to create dev token for cluster {cluster}");
        let url = format!("{}/api/{}/dev-token", *AI_API_BASE_URL, cluster);

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        response
            .json::<GatewayToken>()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())
    }

    async fn create_dev_enrollment_ticket(
        &self,
        ctx: &Context,
        cluster: Option<&str>,
    ) -> miette::Result<String> {
        let cluster = match cluster {
            Some(c) => c.to_string(),
            None => self.get_cluster(ctx).await?.into_inner(),
        };
        let base_error = || miette!("Failed to create dev enrollment ticket for cluster {cluster}");
        let url = format!("{}/api/{}/dev-ticket", *AI_API_BASE_URL, cluster);

        let client = build_http_client().wrap_err(base_error())?;
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .into_diagnostic()
            .wrap_err("Failed to send request")
            .wrap_err(base_error())?;

        if !response.status().is_success() {
            return Err(miette::miette!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )
            .wrap_err(base_error()));
        }

        #[derive(serde::Deserialize)]
        struct TicketResponse {
            ticket: String,
        }

        let ticket_response: TicketResponse = response
            .json()
            .await
            .into_diagnostic()
            .wrap_err("Failed to parse response")
            .wrap_err(base_error())?;

        Ok(ticket_response.ticket)
    }
}

fn build_http_client() -> miette::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5 * 60))
        .build()
        .into_diagnostic()
        .wrap_err("Failed to build HTTP client")
}
