use crate::colors::{color_primary, OckamColor};
use crate::date::UtcDateTime;
use crate::orchestrator::{ControllerClient, HasSecureClient};
use crate::output::Output;
use crate::terminal::fmt;
use colorful::{Colorful, RGB};
use minicbor::{decode, encode, CborLen, Decode, Decoder, Encode};
use ockam_core::api::{Error, Reply, Request, Status};
use ockam_core::{self, async_trait, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;
use strum::{Display, EnumString};

const API_SERVICE: &str = "subscriptions";

pub const SUBSCRIPTION_PAGE: &str = "https://orchestrator.ockam.io";

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

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[cbor(map)]
pub struct Subscription {
    #[n(1)]
    pub name: SubscriptionName,
    #[n(2)]
    pub is_free_trial: bool,
    #[n(3)]
    pub marketplace: Option<String>,
    #[n(4)]
    pub start_date: Option<UtcDateTime>,
    #[n(5)]
    pub end_date: Option<UtcDateTime>,
}

impl Subscription {
    pub fn new(
        name: SubscriptionName,
        is_free_trial: bool,
        marketplace: Option<String>,
        start_date: Option<UtcDateTime>,
        end_date: Option<UtcDateTime>,
    ) -> Self {
        Self {
            name,
            is_free_trial,
            marketplace,
            start_date,
            end_date,
        }
    }

    pub fn end_date(&self) -> Option<UtcDateTime> {
        self.end_date.clone()
    }

    pub fn start_date(&self) -> Option<UtcDateTime> {
        self.start_date.clone()
    }

    /// A subscription is valid if:
    ///    - is in a trial period and end date is in the future
    ///    - a plan is not in trial status
    pub fn is_valid(&self) -> bool {
        if self.is_free_trial {
            self.end_date()
                .map(|end_date| end_date.is_in_the_future())
                .unwrap_or(false)
        } else {
            true
        }
    }

    pub fn grace_period_end_date(&self) -> crate::Result<Option<UtcDateTime>> {
        if !self.is_free_trial {
            return Ok(None);
        }
        match self.end_date.as_ref() {
            Some(end_date) => {
                let grace_period = time::Duration::days(3);
                let end_date = end_date.clone().into_inner() + grace_period;
                Ok(Some(UtcDateTime::new(end_date)?))
            }
            None => Ok(None),
        }
    }
}

impl Display for Subscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let trial_text = if self.is_free_trial {
            "Trial of the "
        } else {
            ""
        };
        writeln!(f, "{}{} Subscription", trial_text, self.name.colored())?;

        if let (Some(start_date), Some(end_date)) = (self.start_date(), self.end_date()) {
            writeln!(
                f,
                "{}Started on {}, expires in {}",
                fmt::INDENTATION,
                color_primary(start_date.format_human()?),
                color_primary(end_date.diff_human(&start_date)),
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Display, EnumString)]
pub enum SubscriptionName {
    #[strum(to_string = "Gold", ascii_case_insensitive)]
    Gold,
    #[strum(to_string = "Silver", ascii_case_insensitive)]
    Silver,
    #[strum(to_string = "Bronze", ascii_case_insensitive)]
    Bronze,
    #[strum(
        to_string = "Basic",
        serialize = "basic",
        serialize = "developer-premium",
        serialize = "developer-free",
        ascii_case_insensitive
    )]
    Basic,
    #[strum(default, to_string = "{0}", ascii_case_insensitive)]
    Other(String),
}

impl<C> Encode<C> for SubscriptionName {
    fn encode<W: encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> std::result::Result<(), encode::Error<W::Error>> {
        self.to_string().encode(e, ctx)
    }
}

impl<C> CborLen<C> for SubscriptionName {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.to_string().cbor_len(ctx)
    }
}

impl<'b, C> Decode<'b, C> for SubscriptionName {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> std::result::Result<Self, decode::Error> {
        SubscriptionName::from_str(&String::decode(d, ctx)?)
            .map_err(|_| decode::Error::message("Invalid subscription name"))
    }
}

impl SubscriptionName {
    pub fn colored(&self) -> String {
        let color = match self {
            SubscriptionName::Gold => RGB::new(255, 215, 0),
            SubscriptionName::Silver => RGB::new(230, 232, 250),
            SubscriptionName::Bronze => RGB::new(140, 120, 83),
            _ => OckamColor::PrimaryResource.color(),
        };
        self.to_string().color(color).to_string()
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
        trace!(space_id = ?req_body.space_id, space_name = ?req_body.space_name, "activating subscription");
        let req = Request::post("/v0/activate").body(req_body);
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id))]
    async fn unsubscribe(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(subscription = %subscription_id, "unsubscribing");
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
        trace!(subscription = %subscription_id, "updating subscription contact info");
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
        trace!(subscription = %subscription_id, new_space_id = %new_space_id, "updating subscription space");
        let req = Request::put(format!("/v0/{subscription_id}/space_id")).body(new_space_id);
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all)]
    async fn get_subscriptions(&self, ctx: &Context) -> Result<Reply<Vec<SubscriptionLegacy>>> {
        trace!("listing subscriptions");
        let req = Request::get("/v0/");
        self.get_secure_client().ask(ctx, API_SERVICE, req).await
    }

    #[instrument(skip_all, fields(subscription_id = subscription_id))]
    async fn get_subscription(
        &self,
        ctx: &Context,
        subscription_id: String,
    ) -> Result<Reply<SubscriptionLegacy>> {
        trace!(subscription = %subscription_id, "getting subscription");
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
    use std::str::FromStr;

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
                name: SubscriptionName::arbitrary(g),
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

    impl Arbitrary for SubscriptionName {
        fn arbitrary(g: &mut Gen) -> Self {
            match u8::arbitrary(g) % 4 {
                0 => SubscriptionName::Gold,
                1 => SubscriptionName::Silver,
                2 => SubscriptionName::Bronze,
                _ => SubscriptionName::Basic,
            }
        }
    }

    #[test]
    fn test_subscription_name_parsing() {
        let cases = [
            ("Gold", "Gold", SubscriptionName::Gold),
            ("gold", "Gold", SubscriptionName::Gold),
            ("Silver", "Silver", SubscriptionName::Silver),
            ("silver", "Silver", SubscriptionName::Silver),
            ("Bronze", "Bronze", SubscriptionName::Bronze),
            ("bronze", "Bronze", SubscriptionName::Bronze),
            ("Basic", "Basic", SubscriptionName::Basic),
            ("basic", "Basic", SubscriptionName::Basic),
            ("Developer-Premium", "Basic", SubscriptionName::Basic),
            ("developer-premium", "Basic", SubscriptionName::Basic),
            ("Developer-Free", "Basic", SubscriptionName::Basic),
            ("developer-free", "Basic", SubscriptionName::Basic),
            (
                "FreeText",
                "FreeText",
                SubscriptionName::Other("FreeText".to_string()),
            ),
        ];
        for (from_str, to_string, expected) in cases.into_iter() {
            assert_eq!(SubscriptionName::from_str(from_str).unwrap(), expected);
            assert_eq!(expected.to_string(), to_string);
        }
    }
}
