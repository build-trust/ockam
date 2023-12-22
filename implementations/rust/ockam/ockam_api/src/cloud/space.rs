use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;

use crate::cloud::ControllerClient;
use crate::nodes::InMemoryNode;

const TARGET: &str = "ockam_api::cloud::space";

#[derive(Encode, Decode, Serialize, Debug, Clone, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space {
    #[n(1)] pub id: String,
    #[n(2)] pub name: String,
    #[n(3)] pub users: Vec<String>,
}

impl Space {
    pub fn space_id(&self) -> String {
        self.id.clone()
    }

    pub fn space_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Encode, Decode, Debug)]
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
}

#[async_trait]
impl Spaces for InMemoryNode {
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
            )
            .await?;
        Ok(space)
    }

    async fn get_space(&self, ctx: &Context, space_id: &str) -> miette::Result<Space> {
        let controller = self.create_controller().await?;
        let space = controller.get_space(ctx, space_id).await?;
        self.cli_state
            .store_space(
                &space.id,
                &space.name,
                space.users.iter().map(|u| u.as_ref()).collect(),
            )
            .await?;
        Ok(space)
    }

    async fn get_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<Space> {
        let space_id = self
            .cli_state
            .get_space_by_name(space_name)
            .await?
            .space_id();
        self.get_space(ctx, &space_id).await
    }

    async fn delete_space(&self, ctx: &Context, space_id: &str) -> miette::Result<()> {
        let controller = self.create_controller().await?;
        controller.delete_space(ctx, space_id).await?;
        self.cli_state.delete_space(space_id).await?;
        Ok(())
    }

    async fn delete_space_by_name(&self, ctx: &Context, space_name: &str) -> miette::Result<()> {
        let space_id = self
            .cli_state
            .get_space_by_name(space_name)
            .await?
            .space_id();
        self.delete_space(ctx, &space_id).await
    }

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
        self.secure_client
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn get_space(&self, ctx: &Context, space_id: &str) -> miette::Result<Space> {
        trace!(target: TARGET, space = %space_id, "getting space");
        let req = Request::get(format!("/v0/{space_id}"));
        self.secure_client
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn delete_space(&self, ctx: &Context, space_id: &str) -> miette::Result<()> {
        trace!(target: TARGET, space = %space_id, "deleting space");
        let req = Request::delete(format!("/v0/{space_id}"));
        self.secure_client
            .tell(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn list_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>> {
        trace!(target: TARGET, "listing spaces");
        self.secure_client
            .ask(ctx, "spaces", Request::get("/v0/"))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
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
