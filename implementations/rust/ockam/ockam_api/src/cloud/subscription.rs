use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

use crate::nodes::NodeManager;

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
    use super::*;
    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    const TARGET: &str = "ockam_api::cloud::subscription";
    const API_SERVICE: &str = "subscriptions";

    impl NodeManagerWorker {
        pub(crate) async fn unsubscribe(&mut self, ctx: &mut Context, id: &str) -> Result<Vec<u8>> {
            trace!(target: TARGET, subscription = %id, "unsubscribing");
            let req_builder = Request::put(format!("/v0/{id}/unsubscribe"));

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }

        pub(crate) async fn update_subscription_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            trace!(target: TARGET, subscription = %id, "updating subscription space");
            let req_builder = Request::put(format!("/v0/{id}/space_id")).body(req_wrapper.req);

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
        pub(crate) async fn update_subscription_contact_info(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<String> = dec.decode()?;
            trace!(target: TARGET, subscription = %id, "updating subscription contact info");
            let req_builder = Request::put(format!("/v0/{id}/contact_info")).body(req_wrapper.req);

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
        pub(crate) async fn list_subscriptions(&mut self, ctx: &mut Context) -> Result<Vec<u8>> {
            trace!(target: TARGET, "listing subscriptions");
            let req_builder = Request::get("/v0/");

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
        pub(crate) async fn get_subscription(
            &mut self,
            ctx: &mut Context,
            id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, subscription = %id, "getting subscription");
            let req_builder = Request::get(format!("/v0/{id}"));

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
        pub(crate) async fn activate_subscription(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<ActivateSubscription> = dec.decode()?;
            let req_body = req_wrapper.req;
            trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");
            let req_builder = Request::post("/v0/activate").body(req_body);

            self.request_controller(ctx, API_SERVICE, req_builder, None)
                .await
        }
    }

    impl NodeManager {
        pub async fn activate_subscription(
            &self,
            ctx: &Context,
            space_id: String,
            subscription_data: String,
        ) -> Result<Subscription> {
            let req_body = ActivateSubscription::existing(space_id, subscription_data);
            trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");
            let req_builder = Request::post("/v0/activate").body(req_body);

            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn unsubscribe(
            &self,
            ctx: &Context,
            subscription_id: String,
        ) -> Result<Subscription> {
            trace!(target: TARGET, subscription = %subscription_id, "unsubscribing");
            let req_builder = Request::put(format!("/v0/{subscription_id}/unsubscribe"));
            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn update_subscription_contact_info(
            &self,
            ctx: &Context,
            subscription_id: String,
            contact_info: String,
        ) -> Result<Subscription> {
            trace!(target: TARGET, subscription = %subscription_id, "updating subscription contact info");
            let req_builder =
                Request::put(format!("/v0/{subscription_id}/contact_info")).body(contact_info);

            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn update_subscription_space(
            &self,
            ctx: &Context,
            subscription_id: String,
            new_space_id: String,
        ) -> Result<Subscription> {
            trace!(target: TARGET, subscription = %subscription_id, new_space_id = %new_space_id, "updating subscription space");
            let req_builder =
                Request::put(format!("/v0/{subscription_id}/space_id")).body(new_space_id);

            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn get_subscriptions(&self, ctx: &Context) -> Result<Vec<Subscription>> {
            trace!(target: TARGET, "listing subscriptions");
            let req_builder = Request::get("/v0/");

            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn get_subscription(
            &self,
            ctx: &Context,
            subscription_id: String,
        ) -> Result<Option<Subscription>> {
            trace!(target: TARGET, subscription = %subscription_id, "getting subscription");
            let req_builder = Request::get(format!("/v0/{subscription_id}"));
            let bytes = self
                .request_controller(ctx, API_SERVICE, req_builder, None)
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub async fn get_subscription_by_space_id(
            &self,
            ctx: &Context,
            space_id: String,
        ) -> Result<Option<Subscription>> {
            let subscriptions: Vec<Subscription> = self.get_subscriptions(ctx).await?;
            let subscription = subscriptions
                .into_iter()
                .find(|s| s.space_id == Some(space_id.clone()));
            Ok(subscription)
        }
    }
}
