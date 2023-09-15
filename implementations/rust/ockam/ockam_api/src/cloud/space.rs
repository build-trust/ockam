use crate::cloud::Controller;
use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use ockam_core::api::Request;
use ockam_core::async_trait;
use ockam_node::Context;
use serde::Serialize;

const TARGET: &str = "ockam_api::cloud::space";

#[derive(Encode, Decode, Serialize, Debug, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space {
    #[n(1)] pub id: String,
    #[n(2)] pub name: String,
    #[n(3)] pub users: Vec<String>,
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
        name: String,
        users: Vec<String>,
    ) -> miette::Result<Space>;

    async fn get_space(&self, ctx: &Context, space_id: String) -> miette::Result<Space>;

    async fn delete_space(&self, ctx: &Context, space_id: String) -> miette::Result<()>;

    async fn list_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>>;
}

#[async_trait]
impl Spaces for Controller {
    async fn create_space(
        &self,
        ctx: &Context,
        name: String,
        users: Vec<String>,
    ) -> miette::Result<Space> {
        trace!(target: TARGET, space = %name, "creating space");
        let req = Request::post("/v0/").body(CreateSpace::new(name, users));
        self.0
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn get_space(&self, ctx: &Context, space_id: String) -> miette::Result<Space> {
        trace!(target: TARGET, space = %space_id, "getting space");
        let req = Request::get(format!("/v0/{space_id}"));
        self.0
            .ask(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn delete_space(&self, ctx: &Context, space_id: String) -> miette::Result<()> {
        trace!(target: TARGET, space = %space_id, "deleting space");
        let req = Request::delete(format!("/v0/{space_id}"));
        self.0
            .tell(ctx, "spaces", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    async fn list_spaces(&self, ctx: &Context) -> miette::Result<Vec<Space>> {
        trace!(target: TARGET, "listing spaces");
        self.0
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
