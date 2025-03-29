use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use tracing::Level;

use crate::orchestrator::operation::CreateOperationResponse;
use crate::orchestrator::project::models::{InfluxDBTokenLeaseManagerConfig, OktaConfig};
use crate::orchestrator::{ControllerClient, HasSecureClient};
use crate::output::Output;
use crate::Result;
use ockam::Message;
use ockam_core::api::Request;
use ockam_core::{async_trait, cbor_encode_preallocate, Decodable, Encodable, Encoded};
use ockam_node::Context;

const API_SERVICE: &str = "projects";

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Message)]
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

impl Encodable for Addon {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for Addon {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
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

#[derive(Encode, Decode, CborLen, Debug, Message)]
#[cbor(transparent)]
pub struct AddonList(#[n(0)] pub Vec<Addon>);

impl Encodable for AddonList {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for AddonList {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
}
#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct KafkaConfig {
    #[serde(skip)]
    #[cbor(n(1))] pub bootstrap_server: String,
}

impl Encodable for KafkaConfig {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for KafkaConfig {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
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

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Message)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DisableAddon {
    #[cbor(n(1))] pub addon_id: String,
}

impl Encodable for DisableAddon {
    fn encode(self) -> ockam_core::Result<Encoded> {
        cbor_encode_preallocate(self)
    }
}

impl Decodable for DisableAddon {
    fn decode(e: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(e)?)
    }
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
    #[instrument(skip_all, fields(project_id = project_id), level = Level::TRACE)]
    async fn list_addons(&self, ctx: &Context, project_id: &str) -> miette::Result<Vec<Addon>> {
        trace!(project_id, "listing addons");
        let req = Request::get(format!("/v0/{project_id}/addons"));
        let addon_list: AddonList = self
            .get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("list addons")?;
        Ok(addon_list.0)
    }

    #[instrument(skip_all, fields(project_id = project_id), level = Level::TRACE)]
    async fn configure_confluent_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: KafkaConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(project_id, "configuring kafka addon");
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

    #[instrument(skip_all, fields(project_id = project_id), level = Level::TRACE)]
    async fn configure_okta_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: OktaConfig,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(project_id, "configuring okta addon");
        let req =
            Request::post(format!("/v1/projects/{project_id}/configure_addon/okta")).body(config);
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("configure okta addon")
    }

    #[instrument(skip_all, fields(project_id = project_id), level = Level::TRACE)]
    async fn configure_influxdb_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        config: InfluxDBTokenLeaseManagerConfig,
    ) -> miette::Result<CreateOperationResponse> {
        //
        trace!(project_id, "configuring influxdb addon");
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

    #[instrument(skip_all, fields(project_id = project_id, addon_id = addon_id), level = Level::TRACE)]
    async fn disable_addon(
        &self,
        ctx: &Context,
        project_id: &str,
        addon_id: &str,
    ) -> miette::Result<CreateOperationResponse> {
        trace!(project_id, "disabling addon");
        let req = Request::post(format!("/v1/projects/{project_id}/disable_addon"))
            .body(DisableAddon::new(addon_id));
        self.get_secure_client()
            .ask(ctx, API_SERVICE, req)
            .await
            .into_diagnostic()?
            .miette_success("disable addon")
    }
}
