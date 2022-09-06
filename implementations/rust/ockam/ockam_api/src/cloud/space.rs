use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] pub tag: TypeTag<7574645>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub name: CowStr<'a>,
    #[b(3)] pub users: Vec<CowStr<'a>>,
}

impl Clone for Space<'_> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl Space<'_> {
    pub fn to_owned<'r>(&self) -> Space<'r> {
        Space {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            id: self.id.to_owned(),
            name: self.name.to_owned(),
            users: self.users.iter().map(|x| x.to_owned()).collect(),
        }
    }
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSpace<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<3888657>,
    #[b(1)] pub name: CowStr<'a>,
    #[b(2)] pub users: Vec<CowStr<'a>>,
}

impl<'a> CreateSpace<'a> {
    pub fn new<S: Into<CowStr<'a>>, T: AsRef<str>>(name: S, users: &'a [T]) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
            users: users.iter().map(|x| CowStr::from(x.as_ref())).collect(),
        }
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::space::CreateSpace;
    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeManager;

    const TARGET: &str = "ockam_api::cloud::space";

    impl NodeManager {
        pub(crate) async fn create_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateSpace> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "create_space";
            trace!(target: TARGET, space = %req_body.name, "creating space");

            let req_builder = Request::post("/v0/").body(req_body);
            self.request_controller(
                ctx,
                label,
                "create_space",
                cloud_route,
                "spaces",
                req_builder,
            )
            .await
        }

        pub(crate) async fn list_spaces(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_spaces";
            trace!(target: TARGET, "listing spaces");

            let req_builder = Request::get("/v0/");
            self.request_controller(ctx, label, None, cloud_route, "spaces", req_builder)
                .await
        }

        pub(crate) async fn get_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_space";
            trace!(target: TARGET, space = %id, space = %id, "getting space");

            let req_builder = Request::get(format!("/v0/{id}"));
            self.request_controller(ctx, label, None, cloud_route, "spaces", req_builder)
                .await
        }

        pub(crate) async fn delete_space(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "delete_space";
            trace!(target: TARGET, space = %id, "deleting space");

            let req_builder = Request::delete(format!("/v0/{id}"));
            self.request_controller(ctx, label, None, cloud_route, "spaces", req_builder)
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

        use ockam_core::api::SCHEMA;

        use super::*;

        #[derive(Debug, Clone)]
        struct Sp(Space<'static>);

        impl Arbitrary for Sp {
            fn arbitrary(g: &mut Gen) -> Self {
                Sp(Space {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g).into(),
                    name: String::arbitrary(g).into(),
                    users: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                })
            }
        }

        #[derive(Debug, Clone)]
        struct CSp(CreateSpace<'static>);

        impl Arbitrary for CSp {
            fn arbitrary(g: &mut Gen) -> Self {
                CSp(CreateSpace {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    name: String::arbitrary(g).into(),
                    users: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                })
            }
        }

        quickcheck! {
            fn space(o: Sp) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn spaces(o: Vec<Sp>) -> TestResult {
                let empty: Vec<Space> = vec![];
                let cbor = minicbor::to_vec(&empty).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Space> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(&o).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_space(o: CSp) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }
}
