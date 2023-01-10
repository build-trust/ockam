use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct Addon<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)]
    pub tag: TypeTag<1530077>,
    #[b(1)]
    #[serde(borrow)]
    pub id: CowStr<'a>,
    #[b(2)]
    #[serde(borrow)]
    pub description: CowStr<'a>,
    #[n(3)]
    pub enabled: bool,
}

mod node {
    use minicbor::Decoder;
    use ockam::AsyncTryClone;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::project::{InfluxDBTokenLeaseManagerConfig, OktaConfig};
    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::error::ApiError;
    use crate::nodes::NodeManagerWorker;

    const TARGET: &str = "ockam_api::cloud::addon";
    const API_SERVICE: &str = "projects";

    impl NodeManagerWorker {
        pub(crate) async fn list_addons(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_addons";
            trace!(target: TARGET, project_id, "listing addons");

            let req_builder = Request::get(format!("/v0/{project_id}/addons"));

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                API_SERVICE,
                req_builder,
                ident,
            )
            .await
        }

        pub(crate) async fn configure_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
            addon_id: &str,
        ) -> Result<Vec<u8>> {
            match addon_id {
                "okta" => self.configure_okta_addon(ctx, dec, project_id).await,
                "influxdb_token_lease_manager" => {
                    self.configure_influxdb_token_lease_manager_addon(ctx, dec, project_id)
                        .await
                }
                _ => Err(ApiError::generic(&format!("Unknown addon: {addon_id}"))),
            }
        }

        async fn configure_okta_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<OktaConfig> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "configure_okta_addon";
            trace!(target: TARGET, project_id, "configuring okta addon");

            let req_builder = Request::put(format!("/v0/{project_id}/addons/okta")).body(req_body);

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                API_SERVICE,
                req_builder,
                ident,
            )
            .await
        }

        async fn configure_influxdb_token_lease_manager_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<InfluxDBTokenLeaseManagerConfig> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "configure_influxdb_token_lease_manager_addon";
            trace!(target: TARGET, project_id, "configuring influxdb addon");

            let req_builder = Request::put(format!(
                "/v0/{project_id}/addons/influxdb_token_lease_manager"
            ))
            .body(req_body);

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                API_SERVICE,
                req_builder,
                ident,
            )
            .await
        }

        pub(crate) async fn disable_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
            addon_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "disable_addon";
            trace!(target: TARGET, project_id, addon_id, "disabling addon");

            let req_builder = Request::delete(format!("/v0/{project_id}/addons/{addon_id}"));

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                API_SERVICE,
                req_builder,
                ident,
            )
            .await
        }
    }
}
