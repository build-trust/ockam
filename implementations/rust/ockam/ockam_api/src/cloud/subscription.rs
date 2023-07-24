use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct ActivateSubscription {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<3888657>,
    #[n(1)] pub space_id: Option<String>,
    #[n(2)] pub subscription_data: String,
    #[n(3)] pub space_name: Option<String>,
    #[n(4)] pub owner_emails: Option<Vec<String>>,
}

impl ActivateSubscription {
    /// Activates a subscription for an existing space
    pub fn existing<S: Into<String>>(space_id: S, subscription_data: S) -> Self {
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
    pub fn create<S: Into<String>, T: AsRef<str>>(
        space_name: S,
        owner_emails: &[T],
        subscription_data: S,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            space_id: None,
            subscription_data: subscription_data.into(),
            space_name: Some(space_name.into()),
            owner_emails: Some(owner_emails.iter().map(|x| x.as_ref().into()).collect()),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct Subscription {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)]
    pub tag: TypeTag<3783606>,
    #[n(1)]
    pub id: String,
    #[n(2)]
    marketplace: String,
    #[n(3)]
    pub status: String,
    #[n(4)]
    pub entitlements: String,
    #[n(5)]
    pub metadata: String,
    #[n(6)]
    pub contact_info: String,
    #[n(7)]
    pub space_id: Option<String>,
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
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "unsubscribe";
            trace!(target: TARGET, subscription = %id, "unsubscribing");

            let req_builder = Request::put(format!("/v0/{id}/unsubscribe"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }

        pub(crate) async fn update_subscription_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "list_sbuscriptions";
            trace!(target: TARGET, subscription = %id, "updating subscription space");

            let req_builder = Request::put(format!("/v0/{id}/space_id")).body(req_body);

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
        pub(crate) async fn update_subscription_contact_info(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "update_subscription_contact_info";
            trace!(target: TARGET, subscription = %id, "updating subscription contact info");

            let req_builder = Request::put(format!("/v0/{id}/contact_info")).body(req_body);

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
        pub(crate) async fn list_subscriptions(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "list_subscriptions";
            trace!(target: TARGET, "listing subscriptions");

            let req_builder = Request::get("/v0/");
            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
        pub(crate) async fn get_subscription(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "get_subscription";
            trace!(target: TARGET, subscription = %id, "getting subscription");

            let req_builder = Request::get(format!("/v0/{id}"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
        pub(crate) async fn activate_subscription(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<ActivateSubscription> = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "activate_subscription";
            trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");

            let req_builder = Request::post("/v0/activate").body(req_body);

            self.request_controller(
                ctx,
                label,
                "activate_request",
                &cloud_multiaddr,
                API_SERVICE,
                req_builder,
                None,
            )
            .await
        }
    }
}
