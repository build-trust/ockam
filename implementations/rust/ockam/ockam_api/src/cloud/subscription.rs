use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam_core::api::{Error, Reply, Request, Status};
use ockam_core::{self, async_trait, Result};
use ockam_node::Context;

use crate::cloud::secure_client::SecureClient;

const TARGET: &str = "ockam_api::cloud::subscription";
const API_SERVICE: &str = "subscriptions";

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

#[async_trait]
pub trait Subscriptions {
    async fn activate_subscription(
        &self,
        ctx: &Context,
        space_id: String,
        subscription_data: String,
    ) -> Result<Reply<Subscription>>;

    async fn unsubscribe(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<Subscription>>;

    async fn update_subscription_contact_info(
        &self,
        ctx: &Context,
        subscription_id: String,
        contact_info: String,
    ) -> Result<Reply<Subscription>>;

    async fn update_subscription_space(
        &self,
        ctx: &Context,
        subscription_id: String,
        new_space_id: String,
    ) -> Result<Reply<Subscription>>;

    async fn get_subscriptions(&self, ctx: &Context) -> Result<Reply<Vec<Subscription>>>;

    async fn get_subscription(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<Subscription>>;

    async fn get_subscription_by_space_id(
        &self,
        ctx: &Context,
        space_id: String,
    ) -> Result<Reply<Subscription>>;
}

#[async_trait]
impl Subscriptions for SecureClient {
    async fn activate_subscription(
        &self,
        ctx: &Context,
        space_id: String,
        subscription_data: String,
    ) -> Result<Reply<Subscription>> {
        let req_body = ActivateSubscription::existing(space_id, subscription_data);
        trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");
        let req = Request::post("/v0/activate").body(req_body);
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn unsubscribe(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<Subscription>> {
        trace!(target: TARGET, subscription = %subscription_id, "unsubscribing");
        let req = Request::put(format!("/v0/{subscription_id}/unsubscribe"));
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn update_subscription_contact_info(
        &self,
        ctx: &Context,
        subscription_id: String,
        contact_info: String,
    ) -> Result<Reply<Subscription>> {
        trace!(target: TARGET, subscription = %subscription_id, "updating subscription contact info");
        let req = Request::put(format!("/v0/{subscription_id}/contact_info")).body(contact_info);
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn update_subscription_space(
        &self,
        ctx: &Context,
        subscription_id: String,
        new_space_id: String,
    ) -> Result<Reply<Subscription>> {
        trace!(target: TARGET, subscription = %subscription_id, new_space_id = %new_space_id, "updating subscription space");
        let req = Request::put(format!("/v0/{subscription_id}/space_id")).body(new_space_id);
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn get_subscriptions(&self, ctx: &Context) -> Result<Reply<Vec<Subscription>>> {
        trace!(target: TARGET, "listing subscriptions");
        let req = Request::get("/v0/");
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn get_subscription(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<Subscription>> {
        trace!(target: TARGET, subscription = %subscription_id, "getting subscription");
        let req = Request::get(format!("/v0/{subscription_id}"));
        self.ask(ctx, API_SERVICE, req).await
    }

    async fn get_subscription_by_space_id(
        &self,
        ctx: &Context,
        space_id: String,
    ) -> Result<Reply<Subscription>> {
        let subscriptions: Vec<Subscription> = self.get_subscriptions(ctx).await?.success()?;
        let subscription = subscriptions
            .into_iter()
            .find(|s| s.space_id == Some(space_id.clone()));
        match subscription {
            Some(subscription) => Ok(Reply::Successful(subscription)),
            None => Ok(Reply::Failed(
                Error::new_without_path(),
                Some(Status::NotFound),
            )),
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
