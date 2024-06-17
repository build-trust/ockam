use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::operation::CreateOperationResponse;
use crate::cloud::project::models::{InfluxDBTokenLeaseManagerConfig, OktaConfig};
use crate::cloud::{ControllerClient, HasSecureClient};
use crate::output::Output;
use crate::Result;

const TARGET: &str = "ockam_api::cloud::addon";
const API_SERVICE: &str = "projects";

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct Addon {
    #[n(1)]
    pub id: String,
    #[n(2)]
    pub description: String,
    #[n(3)]
    pub enabled: bool,
}

impl Output for Addon {
    fn item(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Addon:")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Enabled: {}", self.enabled)?;
        write!(w, "\n  Description: {}", self.description)?;
        writeln!(w)?;
        Ok(w)
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct KafkaConfig {
    #[serde(skip)]
    #[cbor(n(1))] pub bootstrap_server: String,
}

impl KafkaConfig {
    pub fn new<S: Into<String>>(bootstrap_server: S) -> Self {
        Self {
            bootstrap_server: bootstrap_server.into(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for KafkaConfig {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            bootstrap_server: String::arbitrary(g),
        }
    }
}

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DisableAddon {
    #[cbor(n(1))] pub addon_id: String,
}

impl DisableAddon {
    pub fn new<S: Into<String>>(addon_id: S) -> Self {
        Self {
            addon_id: addon_id.into(),
        }
    }
}

#[async_trait]
pub trait Addons {
    async fn list_addons(&self, ctx: &Context, project_id: &str) -> miette::Result<Vec<Addon>>;

    async fn configure_confluent_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: KafkaConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn configure_okta_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: OktaConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn configure_influxdb_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: InfluxDBTokenLeaseManagerConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn disable_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        addon_id: &str,
    ) -> miette::Result<CreateOperationResponse>;
}

#[async_trait]
impl Addons for ControllerClient {
    #[instrument(skip_all, fields(project_id = project_id))]
    async fn list_addons(&self, ctx: &Context, project_id: &str) -> miette::Result<Vec<Addon>> {
        trace!(target: TARGET, project_id, "listing addons");
        let req = Request::get(format!("/v0/{project_id}/addons"));
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("list addons")
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    async fn configure_confluent_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: KafkaConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "configuring kafka addon");
        let req = Request::post(format!(
            "/v1/projects/{project_id}/configure_addon/confluent"
        ))
        .body(config);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("configure kafka addon")
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    async fn configure_okta_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: OktaConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "configuring okta addon");
        let req =
            Request::post(format!("/v1/projects/{project_id}/configure_addon/okta")).body(config);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("configure okta addon")
    }

    #[instrument(skip_all, fields(project_id = project_id))]
    async fn configure_influxdb_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: InfluxDBTokenLeaseManagerConfig,
    ) -> miette::Result<CreateOperationResponse> {
        //
        trace!(target: TARGET, project_id, "configuring influxdb addon");
        let req = Request::post(format!(
            "/v1/projects/{project_id}/configure_addon/influxdb_token_lease_manager"
        ))
        .body(config);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("configure influxdb addon")
    }

    #[instrument(skip_all, fields(project_id = project_id, addon_id = addon_id))]
    async fn disable_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        addon_id: &str,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "disabling addon");
        let req = Request::post(format!("/v1/projects/{project_id}/disable_addon"))
            .body(DisableAddon::new(addon_id));
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("disable addon")
    }
}
