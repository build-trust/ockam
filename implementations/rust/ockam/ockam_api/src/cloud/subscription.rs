use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::nodes::NodeManager;

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct ActivateSubscription {
    #[n(1)] pub space_id: Option<String>,
    #[n(2)] pub subscription_data: String,
    #[n(3)] pub space_name: Option<String>,
    #[n(4)] pub owner_emails: Option<Vec<String>>,
}

impl ActivateSubscription {
    /// Activates a subscription for an existing space
    pub fn existing<S: Into<String>>(space_id: S, subscription_data: S) -> Self {
        Self {
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

#[cfg(test)]
pub mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use crate::schema::tests::validate_with_schema;

    use super::*;

    quickcheck! {
        fn subcription(s: Subscription) -> TestResult {
            validate_with_schema("subscription", s)
        }

        fn activate_subcription(s: ActivateSubscription) -> TestResult {
            validate_with_schema("activate_subscription", s)
        }
    }

    impl Arbitrary for Subscription {
        fn arbitrary(g: &mut Gen) -> Self {
            Subscription {
                id: String::arbitrary(g),
                marketplace: String::arbitrary(g),
                status: String::arbitrary(g),
                entitlements: String::arbitrary(g),
                metadata: String::arbitrary(g),
                contact_info: String::arbitrary(g),
                space_id: bool::arbitrary(g).then(|| String::arbitrary(g)),
            }
        }
    }

    impl Arbitrary for ActivateSubscription {
        fn arbitrary(g: &mut Gen) -> Self {
            ActivateSubscription::create(
                String::arbitrary(g),
                &[String::arbitrary(g), String::arbitrary(g)],
                String::arbitrary(g),
            )
        }
    }
}
