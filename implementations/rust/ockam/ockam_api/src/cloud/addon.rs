use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct Addon {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)]
    pub tag: TypeTag<1530077>,
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
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<1697996>,

    #[cbor(n(1))] pub bootstrap_server: String,
}

impl ConfluentConfig {
    pub fn new<S: Into<String>>(bootstrap_server: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            bootstrap_server: bootstrap_server.into(),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ConfluentConfigResponse {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<6434816>,

    #[cbor(n(1))] pub bootstrap_server: String,
}

impl ConfluentConfigResponse {
    pub fn new<S: ToString>(bootstrap_server: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            bootstrap_server: bootstrap_server.to_string(),
        }
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for ConfluentConfigResponse {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            bootstrap_server: String::arbitrary(g),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DisableAddon {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<8677807>,

    #[cbor(n(1))] pub addon_id: String,
}

impl DisableAddon {
    pub fn new<S: Into<String>>(addon_id: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addon_id: addon_id.into(),
        }
    }
}

mod node {
    use minicbor::{Decode, Decoder, Encode};
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::addon::{ConfluentConfig, DisableAddon};
    use crate::cloud::project::{InfluxDBTokenLeaseManagerConfig, OktaConfig};
    use crate::cloud::CloudRequestWrapper;
    use crate::error::ApiError;
    use crate::nodes::NodeManagerWorker;

    const TARGET: &str = "ockam_api::cloud::addon";
    const API_SERVICE: &str = "projects";

    impl NodeManagerWorker {
        pub(crate) async fn list_addons(
            &mut self,
            ctx: &mut Context,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, project_id, "listing addons");
            let req_builder = Request::get(format!("/v0/{project_id}/addons"));

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }

        pub(crate) async fn configure_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
            addon_id: &str,
        ) -> Result<Vec<u8>> {
            // TODO: Add on ids should not be magic strings
            match addon_id {
                "okta" => {
                    self.configure_addon_impl::<OktaConfig>(ctx, dec, project_id, addon_id)
                        .await
                }
                "influxdb_token_lease_manager" => {
                    self.configure_addon_impl::<InfluxDBTokenLeaseManagerConfig>(
                        ctx, dec, project_id, addon_id,
                    )
                    .await
                }
                "confluent" => {
                    self.configure_addon_impl::<ConfluentConfig>(ctx, dec, project_id, addon_id)
                        .await
                }
                _ => Err(ApiError::core(format!("Unknown addon: {addon_id}"))),
            }
        }

        async fn configure_addon_impl<'a, T: Encode<()> + Decode<'a, ()>>(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'a>,
            project_id: &str,
            addon_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, project_id, addon_id, "configuring addon");
            let req_wrapper: CloudRequestWrapper<T> = dec.decode()?;
            let req_builder = Request::post(format!(
                "/v1/projects/{project_id}/configure_addon/{addon_id}"
            ))
            .body(req_wrapper.req);

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }

        pub(crate) async fn disable_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, project_id, "disabling addon");
            let req_wrapper: CloudRequestWrapper<DisableAddon> = dec.decode()?;
            let req_body = req_wrapper.req;
            let req_builder =
                Request::post(format!("/v1/projects/{project_id}/disable_addon")).body(req_body);

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
    }
}
