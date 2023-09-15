use crate::cloud::operation::CreateOperationResponse;
use crate::cloud::project::{InfluxDBTokenLeaseManagerConfig, OktaConfig};
use crate::cloud::Controller;
use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;
use serde::{Deserialize, Serialize};

const TARGET: &str = "ockam_api::cloud::addon";
const API_SERVICE: &str = "projects";

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
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

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ConfluentConfig {
    #[cbor(n(1))] pub bootstrap_server: String,
}

impl ConfluentConfig {
    pub fn new<S: Into<String>>(bootstrap_server: S) -> Self {
        Self {
            bootstrap_server: bootstrap_server.into(),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ConfluentConfigResponse {
    #[cbor(n(1))] pub bootstrap_server: String,
}

impl ConfluentConfigResponse {
    pub fn new<S: ToString>(bootstrap_server: S) -> Self {
        Self {
            bootstrap_server: bootstrap_server.to_string(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for ConfluentConfigResponse {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            bootstrap_server: String::arbitrary(g),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
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
    async fn list_addons(&self, ctx: &Context, project_id: String) -> miette::Result<Vec<Addon>>;

    async fn configure_confluent_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: ConfluentConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn configure_okta_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: OktaConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn configure_influxdb_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: InfluxDBTokenLeaseManagerConfig,
    ) -> miette::Result<CreateOperationResponse>;

    async fn disable_addon(
        &self,
        ctx: &Context,
        project_id: String,
        addon_id: String,
    ) -> miette::Result<CreateOperationResponse>;
}

#[async_trait]
impl Addons for Controller {
    async fn list_addons(&self, ctx: &Context, project_id: String) -> miette::Result<Vec<Addon>> {
        trace!(target: TARGET, project_id, "listing addons");
        let req = Request::get(format!("/v0/{project_id}/addons"));
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn configure_confluent_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: ConfluentConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "configuring confluent addon");
        let req = Request::post(format!(
            "/v1/projects/{project_id}/configure_addon/confluent"
        ))
        .body(config);
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn configure_okta_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: OktaConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "configuring okta addon");
        let req =
            Request::post(format!("/v1/projects/{project_id}/configure_addon/okta")).body(config);
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn configure_influxdb_addon(
        &self,
        ctx: &Context,
        project_id: String,
        config: InfluxDBTokenLeaseManagerConfig,
    ) -> miette::Result<CreateOperationResponse> {
        //
        trace!(target: TARGET, project_id, "configuring influxdb addon");
        let req = Request::post(format!(
            "/v1/projects/{project_id}/configure_addon/influxdb_token_lease_manager"
        ))
        .body(config);
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn disable_addon(
        &self,
        ctx: &Context,
        project_id: String,
        addon_id: String,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(target: TARGET, project_id, "disabling addon");
        let req = Request::post(format!("/v1/projects/{project_id}/disable_addon"))
            .body(DisableAddon::new(addon_id));
        self.0
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}
