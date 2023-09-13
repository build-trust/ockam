use minicbor::{Decode, Encode};
use serde::Serialize;

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

mod node {
    use tracing::trace;

    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::space::{CreateSpace, Space};
    use crate::cloud::CloudRequestWrapper;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    const TARGET: &str = "ockam_api::cloud::space";

    impl NodeManager {
        pub async fn create_space(&self, ctx: &Context, req: CreateSpace) -> Result<Space> {
            Response::parse_response_body(
                self.create_space_response(ctx, CloudRequestWrapper::new(req))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn create_space_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateSpace>,
        ) -> Result<Vec<u8>> {
            let req_body = req_wrapper.req;
            trace!(target: TARGET, space = %req_body.name, "creating space");
            let req = Request::post("/v0/").body(req_body);
            self.make_controller_client()
                .await?
                .request(ctx, "spaces", req)
                .await
        }

        pub async fn list_spaces(&self, ctx: &Context) -> Result<Vec<Space>> {
            Response::parse_response_body(self.list_spaces_response(ctx).await?.as_slice())
        }

        pub(crate) async fn list_spaces_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            trace!(target: TARGET, "listing spaces");
            let req = Request::get("/v0/");

            self.make_controller_client()
                .await?
                .request(ctx, "spaces", req)
                .await
        }

        pub async fn get_space(&self, ctx: &Context, id: &str) -> Result<Space> {
            Response::parse_response_body(self.get_space_response(ctx, id).await?.as_slice())
        }

        pub(crate) async fn get_space_response(&self, ctx: &Context, id: &str) -> Result<Vec<u8>> {
            trace!(target: TARGET, space = %id, space = %id, "getting space");
            let req = Request::get(format!("/v0/{id}"));
            self.make_controller_client()
                .await?
                .request(ctx, "spaces", req)
                .await
        }
    }

    impl NodeManagerWorker {
        pub(crate) async fn create_space_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateSpace>,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.create_space_response(ctx, req_wrapper).await
        }

        pub(crate) async fn list_spaces_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.list_spaces_response(ctx).await
        }

        pub(crate) async fn get_space_response(&self, ctx: &Context, id: &str) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.get_space_response(ctx, id).await
        }

        pub async fn delete_space(&self, ctx: &Context, id: &str) -> Result<()> {
            let _ = self.delete_space_response(ctx, id).await?;
            Ok(())
        }

        pub(crate) async fn delete_space_response(
            &self,
            ctx: &Context,
            id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, space = %id, "deleting space");
            let req = Request::delete(format!("/v0/{id}"));
            self.controller_client.request(ctx, "spaces", req).await
        }
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
