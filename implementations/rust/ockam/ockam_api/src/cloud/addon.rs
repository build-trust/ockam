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

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct ConfigureAddon<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)]
    pub tag: TypeTag<8647456>,
    #[b(1)]
    #[serde(borrow)]
    pub tenant: CowStr<'a>,
    #[b(2)]
    #[serde(borrow)]
    pub certificate: CowStr<'a>,
}

impl<'a> ConfigureAddon<'a> {
    pub fn new<S: Into<CowStr<'a>>>(tenant: S, certificate: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant: tenant.into(),
            certificate: certificate.into(),
        }
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeManagerWorker;

    use super::*;

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

            let req_builder = Request::get(format!("/v0/{}/addons", project_id));
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }

        pub(crate) async fn configure_addon(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
            addon_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<ConfigureAddon> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "configure_addon";
            trace!(target: TARGET, project_id, addon_id, "configuring addon");

            let req_builder =
                Request::put(format!("/v0/{}/addons/{}", project_id, addon_id)).body(req_body);
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
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

            let req_builder = Request::delete(format!("/v0/{}/addons/{}", project_id, addon_id));
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }
    }
}
