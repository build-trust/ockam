use minicbor::{Decode, Encode};
use serde::Serialize;

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Encode, Decode, Serialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] pub tag: TypeTag<7574645>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub name: CowStr<'a>,
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSpace<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<3888657>,
    #[b(1)] pub name: CowStr<'a>,
}

impl<'a> CreateSpace<'a> {
    pub fn new<S: Into<CowStr<'a>>>(name: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
        }
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::enroll::auth0::Auth0TokenProvider;
    use crate::cloud::space::CreateSpace;
    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeMan;
    use crate::request;
    use crate::{Request, Response, Status};

    const TARGET: &str = "ockam_api::cloud::space";

    impl<A> NodeMan<A>
    where
        A: Auth0TokenProvider,
    {
        pub(crate) async fn create_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateSpace> = dec.decode()?;
            let req_body = req_wrapper.req;
            let cloud_address = req_wrapper.cloud_address;

            let label = "create_space";
            trace!(target: TARGET, space = %req_body.name, "creating space");

            let route = self.api_service_route(&cloud_address, "spaces");
            let req_builder = Request::post("v0/").body(req_body);
            match request(ctx, label, "create_space", route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to create space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }

        pub(crate) async fn list_spaces(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_address = req_wrapper.cloud_address;

            let label = "list_spaces";
            trace!(target: TARGET, "listing spaces");

            let route = self.api_service_route(&cloud_address, "spaces");
            let req_builder = Request::get("v0/");
            match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve spaces");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }

        pub(crate) async fn get_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_address = req_wrapper.cloud_address;

            let label = "get_space";
            trace!(target: TARGET, space = %id, space = %id, "getting space");

            let route = self.api_service_route(&cloud_address, "spaces");
            let req_builder = Request::get(format!("v0/{id}"));
            match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }

        pub(crate) async fn get_space_by_name(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            name: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_address = req_wrapper.cloud_address;

            let label = "get_space_by_name";
            trace!(target: TARGET, space = %name, "getting space");

            let route = self.api_service_route(&cloud_address, "spaces");
            let req_builder = Request::get(format!("v0/name/{name}"));
            match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }

        pub(crate) async fn delete_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_address = req_wrapper.cloud_address;

            let label = "delete_space";
            trace!(target: TARGET, space = %id, "deleting space");

            let route = self.api_service_route(&cloud_address, "spaces");
            let req_builder = Request::delete(format!("v0/{id}"));
            match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use minicbor::{encode, Decoder};
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;

    use crate::cloud::space::CreateSpace;
    use crate::{Method, Request, Response};

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::SCHEMA;

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

    mod node_api {
        use crate::cloud::CloudRequestWrapper;
        use crate::nodes::NodeMan;
        use crate::Status;
        use ockam_core::Address;

        use super::*;

        #[ockam_macros::test]
        async fn basic_api_usage(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create spaces server
            let cloud_server = Address::from_string("spaces".to_string());
            ctx.start_worker(&cloud_server, SpaceServer::default())
                .await?;

            // Create node manager to handle requests
            let route = NodeMan::test_create(ctx).await?;

            // Create space
            let req = CreateSpace::new("s1");
            let mut buf = vec![];
            Request::builder(Method::Post, "v0/spaces")
                .body(CloudRequestWrapper::new(req, &cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let s = dec.decode::<Space>()?;
            assert_eq!(&s.name, "s1");
            let s_id = s.id.to_string();

            // Retrieve it
            let mut buf = vec![];
            Request::builder(Method::Get, format!("v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let s = dec.decode::<Space>()?;
            assert_eq!(&s.id, &s_id);

            // List it
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/spaces")
                .body(CloudRequestWrapper::bare(&cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Space>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &s_id);

            // Remove it
            let mut buf = vec![];
            Request::builder(Method::Delete, format!("v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));

            // Check list returns empty vec
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/spaces")
                .body(CloudRequestWrapper::bare(&cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Space>>()?;
            assert!(list.is_empty());

            // Check that retrieving it returns a not found error
            let mut buf = vec![];
            Request::builder(Method::Get, format!("v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_server))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::NotFound));

            ctx.stop().await
        }
    }

    #[derive(Debug, Default)]
    pub struct SpaceServer(HashMap<String, Space<'static>>);

    #[ockam_core::worker]
    impl Worker for SpaceServer {
        type Message = Vec<u8>;
        type Context = Context;

        async fn handle_message(
            &mut self,
            ctx: &mut Context,
            msg: Routed<Self::Message>,
        ) -> ockam_core::Result<()> {
            let r = self.on_request(msg.as_body())?;
            ctx.send(msg.return_route(), r).await
        }
    }

    impl SpaceServer {
        fn on_request(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            let r = match req.method() {
                Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                    // Get all nodes:
                    ["v0", ""] => Response::ok(req.id())
                        .body(encode::ArrayIter::new(self.0.values()))
                        .to_vec()?,
                    // Get a single node:
                    ["v0", id] => {
                        if let Some(n) = self.0.get(*id) {
                            Response::ok(req.id()).body(n).to_vec()?
                        } else {
                            Response::not_found(req.id()).to_vec()?
                        }
                    }
                    _ => {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                },
                Some(Method::Post) if req.has_body() => {
                    if let Ok(space) = dec.decode::<CreateSpace>() {
                        let obj = Space {
                            #[cfg(feature = "tag")]
                            tag: TypeTag,
                            id: u32::arbitrary(&mut rng).to_string().into(),
                            name: space.name.to_string().into(),
                        };
                        self.0.insert(obj.id.to_string(), obj.clone());
                        Response::ok(req.id()).body(&obj).to_vec()?
                    } else {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                }
                Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                    [_, id] => {
                        if self.0.remove(*id).is_some() {
                            Response::ok(req.id()).to_vec()?
                        } else {
                            Response::not_found(req.id()).to_vec()?
                        }
                    }
                    _ => {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                },
                _ => {
                    error!("Invalid request: {req:#?}");
                    Response::bad_request(req.id()).to_vec()?
                }
            };
            Ok(r)
        }
    }
}
