use minicbor::{Decode, Encode};
use serde::Serialize;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Debug, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] pub tag: TypeTag<7574645>,
    #[n(1)] pub id: String,
    #[n(2)] pub name: String,
    #[n(3)] pub users: Vec<String>,
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSpace {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<2321503>,
    #[n(1)] pub name: String,
    #[n(2)] pub users: Vec<String>,
}

impl CreateSpace {
    pub fn new(name: String, users: Vec<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name,
            users,
        }
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
        pub async fn create_space(
            &self,
            ctx: &Context,
            req: CreateSpace,
            identity_name: Option<String>,
        ) -> Result<Space> {
            Response::parse_response_body(
                self.create_space_response(ctx, CloudRequestWrapper::new(req, identity_name))
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
            let req_builder = Request::post("/v0/").body(req_body);

            self.request_controller(
                ctx,
                "create_space",
                "create_space",
                "spaces",
                req_builder,
                None,
            )
            .await
        }

        pub async fn list_spaces(&self, ctx: &Context) -> Result<Vec<Space>> {
            Response::parse_response_body(self.list_spaces_response(ctx).await?.as_slice())
        }

        pub(crate) async fn list_spaces_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            trace!(target: TARGET, "listing spaces");
            let req_builder = Request::get("/v0/");

            self.request_controller(ctx, "list_spaces", None, "spaces", req_builder, None)
                .await
        }

        pub async fn get_space(&self, ctx: &Context, id: &str) -> Result<Space> {
            Response::parse_response_body(self.get_space_response(ctx, id).await?.as_slice())
        }

        pub(crate) async fn get_space_response(&self, ctx: &Context, id: &str) -> Result<Vec<u8>> {
            trace!(target: TARGET, space = %id, space = %id, "getting space");
            let req_builder = Request::get(format!("/v0/{id}"));

            self.request_controller(ctx, "get_space", None, "spaces", req_builder, None)
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
            let req_builder = Request::delete(format!("/v0/{id}"));

            self.request_controller(ctx, "delete_space", None, "spaces", req_builder, None)
                .await
        }
    }
}

#[cfg(test)]
pub mod tests {
    use quickcheck::{Arbitrary, Gen};

    use crate::cloud::space::CreateSpace;

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::schema::SCHEMA;

        use super::*;

        #[derive(Debug, Clone)]
        struct Sp(Space);

        impl Arbitrary for Sp {
            fn arbitrary(g: &mut Gen) -> Self {
                Sp(Space {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g),
                    name: String::arbitrary(g),
                    users: vec![String::arbitrary(g), String::arbitrary(g)],
                })
            }
        }

        #[derive(Debug, Clone)]
        struct CSp(CreateSpace);

        impl Arbitrary for CSp {
            fn arbitrary(g: &mut Gen) -> Self {
                CSp(CreateSpace {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    name: String::arbitrary(g),
                    users: vec![String::arbitrary(g), String::arbitrary(g)],
                })
            }
        }

        quickcheck! {
            fn space(o: Sp) -> TestResult {
                let cbor = minicbor::to_vec(o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn spaces(o: Vec<Sp>) -> TestResult {
                let empty: Vec<Space> = vec![];
                let cbor = minicbor::to_vec(empty).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Space> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(o).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_space(o: CSp) -> TestResult {
                let cbor = minicbor::to_vec(o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }
}
