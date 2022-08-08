use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

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
    #[b(3)] pub users: Vec<CowStr<'a>>,
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
            users: users
                .iter()
                .map(|x| x.as_ref().to_string())
                .map(CowStr::from)
                .collect(),
        }
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::{Request, Response, Status};
    use ockam_core::{self, Result};
    use ockam_node::api::request;
    use ockam_node::Context;

    use crate::cloud::space::CreateSpace;
    use crate::cloud::space::Space;
    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeManager;

    const TARGET: &str = "ockam_api::cloud::space";

    impl NodeManager {
        pub(crate) async fn create_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateSpace> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "create_space";
            trace!(target: TARGET, space = %req_body.name, "creating space");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(sc.clone(), "spaces");

            let req_builder = Request::post("/v0/").body(req_body);
            let res = match request(ctx, label, "create_space", route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(target: TARGET, ?err, "Failed to create space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn list_spaces(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_spaces";
            trace!(target: TARGET, "listing spaces");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "spaces");

            let req_builder = Request::get("/v0/");
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve spaces");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn get_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_space";
            trace!(target: TARGET, space = %id, space = %id, "getting space");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "spaces");

            let req_builder = Request::get(format!("/v0/{id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn get_space_by_name(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            name: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_space_by_name";
            trace!(target: TARGET, space = %name, "getting space");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "spaces");

            let req_builder = Request::get("/v0/");
            let res = match request(ctx, label, None, route.clone(), req_builder).await {
                Ok(r) => {
                    let mut dec = Decoder::new(&r);
                    let header = dec.decode::<Response>()?;
                    match header.status() {
                        Some(Status::Ok) => {
                            let spaces = dec.decode::<Vec<Space>>()?;
                            let space = spaces.iter().find(|n| n.name == *name).unwrap();
                            let id = &space.id;
                            let req_builder = Request::get(format!("/v0/{id}"));
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
                        _ => {
                            error!("Failed to retrieve spaces");
                            Ok(Response::builder(req.id(), Status::InternalServerError)
                                .body("Failed to retrieve spaces".to_string())
                                .to_vec()?)
                        }
                    }
                }
                Err(err) => {
                    error!(?err, "Failed to retrieve spaces");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn delete_space(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "delete_space";
            trace!(target: TARGET, space = %id, "deleting space");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "spaces");

            let req_builder = Request::delete(format!("/v0/{id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve space");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }
    }
}

#[cfg(test)]
pub mod tests {
    use minicbor::{encode, Decoder};
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::api::{Method, Request, Response};
    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;

    use crate::cloud::space::CreateSpace;

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

    mod node_api {
        use ockam_core::api::Status;
        use ockam_core::route;

        use crate::cloud::CloudRequestWrapper;
        use crate::nodes::NodeManager;
        use crate::route_to_multiaddr;

        use super::*;

        #[ockam_macros::test]
        async fn basic_api_usage(ctx: &mut Context) -> ockam_core::Result<()> {
            //TODO:  what's the purpose of testing what the fake SpaceServer does?

            // Create node manager to handle requests
            let route = NodeManager::test_create(ctx, "spaces", SpaceServer::default()).await?;
            let cloud_route = route_to_multiaddr(&route!["cloud"]).unwrap();

            // Create space
            let req = CreateSpace::new("s1", &["some@test.com"]);
            let mut buf = vec![];
            Request::builder(Method::Post, "v0/spaces")
                .body(CloudRequestWrapper::new(req, &cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let s = dec.decode::<Space>()?;
            assert_eq!(&s.name, "s1");
            let s_id = s.id.to_string();

            // Retrieve it
            let mut buf = vec![];
            Request::builder(Method::Get, format!("/v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let s = dec.decode::<Space>()?;
            assert_eq!(&s.id, &s_id);

            // List it
            let mut buf = vec![];
            Request::builder(Method::Get, "/v0/spaces")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let list = dec.decode::<Vec<Space>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &s_id);

            // Remove it
            let mut buf = vec![];
            Request::builder(Method::Delete, format!("v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));

            // Check list returns empty vec
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/spaces")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let list = dec.decode::<Vec<Space>>()?;
            assert!(list.is_empty());

            // Check that retrieving it returns a not found error
            let mut buf = vec![];
            Request::builder(Method::Get, format!("/v0/spaces/{s_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::NotFound));

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
                            users: space.users.iter().map(|x| x.to_string().into()).collect(),
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
