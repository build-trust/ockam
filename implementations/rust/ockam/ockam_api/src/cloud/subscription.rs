use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct ActivateSubscription<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<3888657>,
    #[b(1)] pub space_id: Option<CowStr<'a>>,
    #[b(2)] pub subscription_data: CowStr<'a>,
    #[b(3)] pub space_name: Option<CowStr<'a>>,
    #[b(4)] pub owner_emails: Option<Vec<CowStr<'a>>>,
}

impl<'a> ActivateSubscription<'a> {
    /// Activates a subscription for an existing space
    pub fn existing<S: Into<CowStr<'a>>>(space_id: S, subscription_data: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            space_id: Some(space_id.into()),
            subscription_data: subscription_data.into(),
            space_name: None,
            owner_emails: None,
        }
    }

    /// Activates a subscription for a space that will be newly created with the given space name
    #[allow(unused)]
    pub fn create<S: Into<CowStr<'a>>, T: AsRef<str>>(
        space_name: S,
        owner_emails: &'a [T],
        subscription_data: S,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            space_id: None,
            subscription_data: subscription_data.into(),
            space_name: Some(space_name.into()),
            owner_emails: Some(
                owner_emails
                    .iter()
                    .map(|x| CowStr::from(x.as_ref()))
                    .collect(),
            ),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct Subscription<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)]
    pub tag: TypeTag<3783606>,
    #[b(1)]
    #[serde(borrow)]
    pub id: CowStr<'a>,
    #[b(2)]
    #[serde(borrow)]
    marketplace: CowStr<'a>,
    #[b(3)]
    #[serde(borrow)]
    pub status: CowStr<'a>,
    #[b(4)]
    #[serde(borrow)]
    pub entitlements: CowStr<'a>,
    #[b(5)]
    #[serde(borrow)]
    pub metadata: CowStr<'a>,
    #[b(6)]
    #[serde(borrow)]
    pub contact_info: CowStr<'a>,
    #[b(7)]
    #[serde(borrow)]
    pub space_id: Option<CowStr<'a>>,
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

    const TARGET: &str = "ockam_api::cloud::subscription";
    const API_SERVICE: &str = "subscriptions";

    impl NodeManagerWorker {
        pub(crate) async fn unsubscribe(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "unsubscribe";
            trace!(target: TARGET, subscription = %id, "unsubscribing");

            let req_builder = Request::put(format!("/v0/{}/unsubscribe", id));
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }

        pub(crate) async fn update_subscription_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "list_sbuscriptions";
            trace!(target: TARGET, subscription = %id, "updating subscription space");

            let req_builder = Request::put(format!("/v0/{}/space_id", id)).body(req_body);
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }
        pub(crate) async fn update_subscription_contact_info(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "update_subscription_contact_info";
            trace!(target: TARGET, subscription = %id, "updating subscription contact info");

            let req_builder = Request::put(format!("/v0/{}/contact_info", id)).body(req_body);
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }
        pub(crate) async fn list_subscriptions(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_subscriptions";
            trace!(target: TARGET, "listing subscriptions");

            let req_builder = Request::get("/v0/");
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }
        pub(crate) async fn get_subscription(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_subscription";
            trace!(target: TARGET, subscription = %id, "getting subscription");

            let req_builder = Request::get(format!("/v0/{}", id));
            self.request_controller(ctx, label, None, cloud_route, API_SERVICE, req_builder)
                .await
        }
        pub(crate) async fn activate_subscription(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<ActivateSubscription> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "activate_subscription";
            trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");

            let req_builder = Request::post("/v0/activate").body(req_body);
            self.request_controller(
                ctx,
                label,
                "activate_request",
                cloud_route,
                API_SERVICE,
                req_builder,
            )
            .await
        }
    }
}
