use crate::cloud::{ControllerClient, HasSecureClient};
use crate::output::Output;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

use crate::colors::color_primary;
use crate::date::parse_date;
use crate::terminal::fmt;
use ockam_core::api::{Error, Reply, Request, Status};
use ockam_core::{self, async_trait, Result};
use ockam_node::Context;

const TARGET: &str = "ockam_api::cloud::subscription";
const API_SERVICE: &str = "subscriptions";

#[derive(Encode, Decode, CborLen, Debug)]
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

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Clone, Debug, Eq)]
#[cbor(map)]
pub struct Subscription {
    #[n(1)]
    pub name: String,
    #[n(2)]
    pub is_free_trial: bool,
    #[n(3)]
    pub marketplace: Option<String>,
    #[n(4)]
    start_date: Option<String>,
    #[n(5)]
    end_date: Option<String>,
}

impl PartialEq for Subscription {
    fn eq(&self, other: &Self) -> bool {
        // Compare the dates using as unix timestamps, using a tolerance of 1 second
        let start_date_eq = match (self.start_date(), other.start_date()) {
            (Some(start_date), Some(other_start_date)) => {
                let start_date = start_date.unix_timestamp();
                let other_start_date = other_start_date.unix_timestamp();
                (start_date - other_start_date).abs() <= 1
            }
            (None, None) => true,
            _ => false,
        };
        self.name == other.name
            && self.is_free_trial == other.is_free_trial
            && self.marketplace == other.marketplace
            && start_date_eq
    }
}

impl Subscription {
    pub fn new(
        name: String,
        is_free_trial: bool,
        marketplace: Option<String>,
        start_date: Option<OffsetDateTime>,
        end_date: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            name,
            is_free_trial,
            marketplace,
            start_date: start_date.and_then(|date| date.format(&Iso8601::DEFAULT).ok()),
            end_date: end_date.and_then(|date| date.format(&Iso8601::DEFAULT).ok()),
        }
    }

    pub fn end_date(&self) -> Option<OffsetDateTime> {
        self.end_date
            .as_ref()
            .and_then(|date| parse_date(date).ok())
    }

    pub fn start_date(&self) -> Option<OffsetDateTime> {
        self.start_date
            .as_ref()
            .and_then(|date| parse_date(date).ok())
    }
}

impl Display for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Subscription: {}", color_primary(&self.name))?;
        if self.is_free_trial {
            writeln!(f, " (free trial)")?;
        } else {
            writeln!(f)?;
        }

        if let (Some(start_date), Some(end_date)) = (self.start_date(), self.end_date()) {
            writeln!(
                f,
                "{}Started at {}, expires at {}",
                fmt::INDENTATION,
                color_primary(start_date.to_string()),
                color_primary(end_date.to_string()),
            )?;
        }

        if let Some(marketplace) = &self.marketplace {
            writeln!(
                f,
                "{}Marketplace: {}",
                fmt::INDENTATION,
                color_primary(marketplace)
            )?;
        }

        Ok(())
    }
}

impl Output for Subscription {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

/// This struct is now deprecated and used only in the legacy [Subscriptions] API endpoints.
/// The commands using this API were already removed, but the Controller still supports it.
/// This struct along with the [Subscriptions] trait can be removed once the Controller stops
/// supporting the legacy API.
#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cbor(map)]
pub struct SubscriptionLegacy {
    #[n(1)]
    pub id: String,
    #[n(2)]
    pub marketplace: String,
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

impl Output for SubscriptionLegacy {
    fn item(&self) -> crate::Result<String> {
        let mut w = String::new();
        write!(w, "{}Id: {}", fmt::PADDING, self.id)?;
        write!(w, "{}Status: {}", fmt::PADDING, self.status)?;
        write!(
            w,
            "{}Space id: {}",
            fmt::PADDING,
            self.space_id.clone().unwrap_or("N/A".to_string())
        )?;
        write!(w, "{}Entitlements: {}", fmt::PADDING, self.entitlements)?;
        write!(w, "{}Metadata: {}", fmt::PADDING, self.metadata)?;
        write!(w, "{}Contact info: {}", fmt::PADDING, self.contact_info)?;
        Ok(w)
    }
}

#[async_trait]
pub trait Subscriptions {
    async fn activate_subscription(
        &self,
        ctx: &Context,
        space_id: String,
        subscription_data: String,
    ) -> Result<Reply<SubscriptionLegacy>>;

    async fn unsubscribe(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>>;

    async fn update_subscription_contact_info(
        &self,
        ctx: &Context,
        subscription_id: String,
        contact_info: String,
    ) -> Result<Reply<SubscriptionLegacy>>;

    async fn update_subscription_space(
        &self,
        ctx: &Context,
        subscription_id: String,
        new_space_id: String,
    ) -> Result<Reply<SubscriptionLegacy>>;

    async fn get_subscriptions(&self, ctx: &Context) -> Result<Reply<Vec<SubscriptionLegacy>>>;

    async fn get_subscription(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>>;

    async fn get_subscription_by_space_id(
        &self,
        ctx: &Context,
        space_id: String,
    ) -> Result<Reply<SubscriptionLegacy>>;
}

#[async_trait]
impl Subscriptions for ControllerClient {
    #[instrument(skip_all, fields(space_id = space_id, subscription_data = subscription_data))]
    async fn activate_subscription(
        &self,
        ctx: &Context,
        space_id: String,
        subscription_data: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        let req_body = ActivateSubscription::existing(space_id, subscription_data);
        trace!(target: TARGET, space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");
        let req = Request::post("/v0/activate").body(req_body);
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id))]
    async fn unsubscribe(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(target: TARGET, subscription = %subscription_id, "unsubscribing");
        let req = Request::put(format!("/v0/{subscription_id}/unsubscribe"));
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id, contact_info = contact_info))]
    async fn update_subscription_contact_info(
        &self,
        ctx: &Context,
        subscription_id: String,
        contact_info: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(target: TARGET, subscription = %subscription_id, "updating subscription contact info");
        let req = Request::put(format!("/v0/{subscription_id}/contact_info")).body(contact_info);
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id, new_space_id = new_space_id))]
    async fn update_subscription_space(
        &self,
        ctx: &Context,
        subscription_id: String,
        new_space_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(target: TARGET, subscription = %subscription_id, new_space_id = %new_space_id, "updating subscription space");
        let req = Request::put(format!("/v0/{subscription_id}/space_id")).body(new_space_id);
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all)]
    async fn get_subscriptions(&self, ctx: &Context) -> Result<Reply<Vec<SubscriptionLegacy>>> {
        trace!(target: TARGET, "listing subscriptions");
        let req = Request::get("/v0/");
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id))]
    async fn get_subscription(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(target: TARGET, subscription = %subscription_id, "getting subscription");
        let req = Request::get(format!("/v0/{subscription_id}"));
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(space_id = space_id))]
    async fn get_subscription_by_space_id(
        &self,
        ctx: &Context,
        space_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        let subscriptions: Vec<SubscriptionLegacy> =
            self.get_subscriptions(ctx).await?.success()?;
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
    use super::*;
    use crate::schema::tests::validate_with_schema;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    quickcheck! {
        fn subcription_legacy(s: SubscriptionLegacy) -> TestResult {
            validate_with_schema("subscription_legacy", s)
        }

        fn activate_subcription(s: ActivateSubscription) -> TestResult {
            validate_with_schema("activate_subscription", s)
        }
    }

    impl Arbitrary for SubscriptionLegacy {
        fn arbitrary(g: &mut Gen) -> Self {
            SubscriptionLegacy {
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

    impl Arbitrary for Subscription {
        fn arbitrary(g: &mut Gen) -> Self {
            Subscription {
                name: String::arbitrary(g),
                is_free_trial: bool::arbitrary(g),
                marketplace: Option::arbitrary(g),
                start_date: Option::arbitrary(g),
                end_date: Option::arbitrary(g),
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
