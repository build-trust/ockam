use miette::IntoDiagnostic;
use minicbor::{CborLen, Decode, Encode};
use serde::Serialize;
use std::fmt::{Display, Formatter, Write};

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::email_address::EmailAddress;
use crate::cloud::project::models::AdminInfo;
use crate::cloud::project::{Project, ProjectsOrchestratorApi};
use crate::cloud::subscription::Subscription;
use crate::cloud::{ControllerClient, HasSecureClient};
use crate::colors::{color_primary, color_uri, color_warn};
use crate::fmt_log;
use crate::nodes::InMemoryNode;
use crate::output::{comma_separated, Output};
use crate::terminal::fmt;

const TARGET: &str = "ockam_api::cloud::space";

#[derive(Encode, Decode, CborLen, Serialize, Debug, Clone, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space {
    #[n(1)] pub id: String,
    #[n(2)] pub name: String,
    #[n(3)] pub users: Vec<String>,
    #[n(4)] pub subscription: Option<Subscription>,
}

impl Space {
    pub fn space_id(&self) -> String {
        self.id.clone()
    }

    pub fn space_name(&self) -> String {
        self.name.clone()
    }

    pub fn has_subscription(&self) -> bool {
        self.subscription.is_some()
    }

    pub fn is_in_free_trial_subscription(&self) -> bool {
        self.subscription.is_none()
            || self
                .subscription
                .as_ref()
                .map(|s| s.is_free_trial)
                .unwrap_or_default()
    }

    pub fn subscription_status_message(&self, space_is_new: bool) -> crate::Result<String> {
        let mut f = String::new();
        if let Some(subscription) = &self.subscription {
            writeln!(
                f,
                "{}",
                fmt_log!(
                    "This Space has a {} Subscription attached to it.",
                    color_primary(&subscription.name)
                )
            )?;
            if subscription.is_free_trial {
                if space_is_new {
                    writeln!(f)?;
                    writeln!(f, "{}", fmt_log!("As a courtesy, we created a temporary Space for you, so you can continue to build.\n"))?;
                    writeln!(
                        f,
                        "{}",
                        fmt_log!(
                            "Please subscribe to an Ockam plan within two weeks {}",
                            color_uri("https://www.ockam.io/pricing")
                        )
                    )?;
                    writeln!(f, "{}", fmt_log!("{}", color_warn("If you don't subscribe in that time, your Space and all associated Projects will be permanently deleted.")))?;
                } else if let (Some(start_date), Some(end_date)) =
                    (&subscription.start_date(), &subscription.end_date())
                {
                    writeln!(f)?;
                    writeln!(
                        f,
                        "{}",
                        fmt_log!(
                            "Your free trial started on {} and will end on {}.\n",
                            start_date,
                            end_date
                        )
                    )?;
                    writeln!(f, "{}", fmt_log!("Please subscribe to an Ockam plan before the trial ends to avoid any service interruptions {}", color_uri("https://www.ockam.io/pricing")))?;
                    writeln!(f, "{}", fmt_log!("{}", color_warn("If you don't subscribe in that time, your Space and all associated Projects will be permanently deleted.")))?;
                }
            }
        } else {
            writeln!(
                f,
                "{}",
                fmt_log!("This Space does not have a Subscription attached to it.")
            )?;
        }
        Ok(f)
    }
}

impl Display for Space {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", color_primary(&self.name))?;
        writeln!(f, "{}Id: {}", fmt::INDENTATION, color_primary(&self.id))?;
        writeln!(
            f,
            "{}Users: {}",
            fmt::INDENTATION,
            comma_separated(&self.users)
        )?;
        if let Some(subscription) = &self.subscription {
            write!(f, "{}", subscription.iter_output().indent())?;
        }
        Ok(())
    }
}

impl Output for Space {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}

#[derive(Encode, Decode, CborLen, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSpace {
    #[n(1)] pub name: String,
    #[n(2)] pub users: Vec<String>,
}

impl CreateSpace {
    pub fn new(name: String, users: Vec<String>) -> Self {
        Self { name, users }
    }
}

#[async_trait]
pub trait Spaces {
    async fn create_space(
        &self,
        ctx: &Context,
        name: &str,
        users: Vec<&str>,
    ) -> miette::Result<Space>;

    async fn get_space(&self, ctx: &Context, space_id: &str) -> miette::Result<Space>;

    async fn get_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<Space>;

    async fn delete_space(&self, ctx: &Context, space_id: &str) -> miette::Result<()>;

    async fn delete_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<()>;

    async fn get_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>>;

    async fn add_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo>;

    async fn list_space_admins(
        &self,
        ctx: &Context,
        space_id: &str,
    ) -> miette::Result<Vec<AdminInfo>>;

    async fn delete_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()>;
}

#[async_trait]
impl Spaces for InMemoryNode {
    #[instrument(skip_all, fields(space_name = name))]
    async fn create_space(
        &self,
        ctx: &Context,
        name: &str,
        users: Vec<&str>,
    ) -> miette::Result<Space> {
        let controller = self.create_controller().await?;
        let space = controller.create_space(ctx, name, users).await?;
        self.cli_state
            .store_space(
                &space.id,
                &space.name,
                space.users.iter().map(|u| u.as_ref()).collect(),
                space.subscription.as_ref(),
            )
            .await?;
        Ok(space)
    }

    #[instrument(skip_all, fields(space_id = space_id))]
    async fn get_space(&self, ctx: &Context, space_id: &str) -> miette::Result<Space> {
        let controller = self.create_controller().await?;
        let space = controller.get_space(ctx, space_id).await?;
        self.cli_state
            .store_space(
                &space.id,
                &space.name,
                space.users.iter().map(|u| u.as_ref()).collect(),
                space.subscription.as_ref(),
            )
            .await?;
        Ok(space)
    }

    #[instrument(skip_all, fields(space_name = space_name))]
    async fn get_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<Space> {
        let space_id = self
            .cli_state
            .get_space_by_name(space_name)
            .await?
            .space_id();
        self.get_space(ctx, &space_id).await
    }

    #[instrument(skip_all, fields(space_id = space_id))]
    async fn delete_space(&self, ctx: &Context, space_id: &str) -> miette::Result<()> {
        let space_projects = self
            .cli_state
            .projects()
            .get_projects()
            .await?
            .into_iter()
            .filter(|p| p.space_id() == space_id)
            .collect::<Vec<Project>>();
        for project in space_projects {
            self.delete_project(ctx, project.space_id(), project.project_id())
                .await?;
        }

        let controller = self.create_controller().await?;
        controller.delete_space(ctx, space_id).await?;
        self.cli_state.delete_space(space_id).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(space_name = space_name))]
    async fn delete_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<()> {
        let space_id = self
            .cli_state
            .get_space_by_name(space_name)
            .await?
            .space_id();
        self.delete_space(ctx, &space_id).await
    }

    #[instrument(skip_all)]
    async fn get_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>> {
        let controller = self.create_controller().await?;
        let spaces = controller.list_spaces(ctx).await?;
        let default_space = self.cli_state.get_default_space().await.ok();
        for space in &spaces {
            self.cli_state
                .store_space(
                    &space.id,
                    &space.name,
                    space.users.iter().map(|u| u.as_ref()).collect(),
                    space.subscription.as_ref(),
                )
                .await?;

            // make sure that an existing space marked as default is still marked as default
            if let Some(default_space) = &default_space {
                if space.id == default_space.id {
                    self.cli_state.set_space_as_default(&space.id).await?;
                };
            }
        }
        Ok(spaces)
    }

    async fn add_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<AdminInfo> {
        let controller = self.create_controller().await?;
        let res = controller.add_space_admin(ctx, space_id, email).await?;
        self.get_space(ctx, space_id).await?;
        Ok(res)
    }

    async fn list_space_admins(
        &self,
        ctx: &Context,
        space_id: &str,
    ) -> miette::Result<Vec<AdminInfo>> {
        let controller = self.create_controller().await?;
        controller.list_space_admins(ctx, space_id).await
    }

    async fn delete_space_admin(
        &self,
        ctx: &Context,
        space_id: &str,
        email: &EmailAddress,
    ) -> miette::Result<()> {
        let controller = self.create_controller().await?;
        controller.delete_space_admin(ctx, space_id, email).await?;
        self.get_space(ctx, space_id).await?;
        Ok(())
    }
}

impl ControllerClient {
    pub async fn create_space(
        &self,
        ctx: &Context,
        name: &str,
        users: Vec<&str>,
    ) -> miette::Result<Space> {
        trace!(target: TARGET, space = %name, "creating space");
        let req = Request::post("/v0/").body(CreateSpace::new(
            name.into(),
            users.iter().map(|u| u.to_string()).collect(),
        ));
        self.get_secure_client()
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .miette_success("create space")
    }

    pub async fn get_space(&self, ctx: &Context, space_id: &str) -> miette::Result<Space> {
        trace!(target: TARGET, space = %space_id, "getting space");
        let req = Request::get(format!("/v0/{space_id}"));
        self.get_secure_client()
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .miette_success("get space")
    }

    pub async fn delete_space(&self, ctx: &Context, space_id: &str) -> miette::Result<()> {
        trace!(target: TARGET, space = %space_id, "deleting space");
        let req = Request::delete(format!("/v0/{space_id}"));
        self.get_secure_client()
            .tell(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .miette_success("delete space")
    }

    pub async fn list_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>> {
        trace!(target: TARGET, "listing spaces");
        self.get_secure_client()
            .ask(ctx, "spaces", Request::get("/v0/"))
            .await
            .into_diagnostic()?
            .miette_success("list spaces")
    }
}

#[cfg(test)]
pub mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use crate::cloud::space::CreateSpace;
    use crate::schema::tests::validate_with_schema;

    use super::*;

    quickcheck! {
        fn space(s: Space) -> TestResult {
            validate_with_schema("space", s)
        }

        fn spaces(ss: Vec<Space>) -> TestResult {
            validate_with_schema("spaces", ss)
        }

        fn create_space(cs: CreateSpace) -> TestResult {
            validate_with_schema("create_space", cs)
        }
    }

    impl Arbitrary for Space {
        fn arbitrary(g: &mut Gen) -> Self {
            Space {
                id: String::arbitrary(g),
                name: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
                subscription: bool::arbitrary(g).then(|| Subscription::arbitrary(g)),
            }
        }
    }

    impl Arbitrary for CreateSpace {
        fn arbitrary(g: &mut Gen) -> Self {
            CreateSpace {
                name: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
            }
        }
    }
}
