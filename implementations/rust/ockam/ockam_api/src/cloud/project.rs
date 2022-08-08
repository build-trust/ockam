use minicbor::{Decode, Encode};
use serde::Serialize;

use ockam_core::CowStr;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Encode, Decode, Serialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Project<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] pub tag: TypeTag<9056532>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub name: CowStr<'a>,
    #[b(3)] pub space_name: CowStr<'a>,
    #[b(4)] pub services: Vec<CowStr<'a>>,
    #[b(5)] pub access_route: CowStr<'a>, //TODO: should be optional, waiting for changes on the elixir side
    #[b(6)] pub users: Vec<CowStr<'a>>,
    #[b(7)] pub space_id: CowStr<'a>,
    #[b(8)] pub identity: Option<CowStr<'a>>,
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateProject<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8669570>,
    #[b(1)] pub name: CowStr<'a>,
    #[b(2)] pub services: Vec<CowStr<'a>>,
    #[b(3)] pub users: Vec<CowStr<'a>>,
}

impl<'a> CreateProject<'a> {
    pub fn new<S: Into<CowStr<'a>>, T: AsRef<str>>(
        name: S,
        users: &'a [T],
        services: &'a [T],
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
            services: services
                .iter()
                .map(|x| x.as_ref().to_string())
                .map(CowStr::from)
                .collect(),
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

    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeManager;

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManager {
        pub(crate) async fn create_project(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            space_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateProject> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "create_project";
            trace!(target: TARGET, %space_id, project_name = %req_body.name, "creating project");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "projects");

            let req_builder = Request::post(format!("/v0/{space_id}")).body(req_body);
            let res = match request(ctx, label, "create_project", route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to create project");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn list_projects(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_projects";
            trace!(target: TARGET, "listing projects");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "projects");

            let req_builder = Request::get("/v0");
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve projects");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn get_project(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_project";
            trace!(target: TARGET, %project_id, "getting project");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "projects");

            let req_builder = Request::get(format!("/v0/{project_id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve project");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn get_project_by_name(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            space_id: &str,
            project_name: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_project_by_name";
            trace!(target: TARGET, %space_id, %project_name, "getting project");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "projects");

            let req_builder = Request::get("/v0");
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => {
                    let mut dec = Decoder::new(&r);
                    let header = dec.decode::<Response>()?;
                    match header.status() {
                        Some(Status::Ok) => {
                            let projects = dec.decode::<Vec<Project>>()?;
                            match projects
                                .iter()
                                .find(|n| n.name == *project_name && n.space_id == *space_id)
                            {
                                Some(project) => Ok(Response::builder(req.id(), Status::Ok)
                                    .body(project)
                                    .to_vec()?),
                                None => Ok(Response::builder(req.id(), Status::NotFound).to_vec()?),
                            }
                        }
                        _ => {
                            error!("Failed to retrieve project");
                            Ok(
                                Response::builder(req.id(), Status::InternalServerError)
                                    .to_vec()?,
                            )
                        }
                    }
                }
                Err(err) => {
                    error!(?err, "Failed to retrieve project");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn delete_project(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "delete_project";
            trace!(target: TARGET, %space_id, %project_id, "deleting project");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "projects");

            let req_builder = Request::delete(format!("/v0/{space_id}/{project_id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve project");
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
mod tests {
    use minicbor::{encode, Decoder};
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::api::{Method, Request, Response};
    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::SCHEMA;

        use super::*;

        #[derive(Debug, Clone)]
        struct Pr(Project<'static>);

        impl Arbitrary for Pr {
            fn arbitrary(g: &mut Gen) -> Self {
                let identity = String::arbitrary(g).into();
                Pr(Project {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g).into(),
                    name: String::arbitrary(g).into(),
                    space_name: String::arbitrary(g).into(),
                    services: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                    access_route: String::arbitrary(g).into(),
                    users: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                    space_id: String::arbitrary(g).into(),
                    identity: g.choose(&[None, Some(identity)]).unwrap().clone(),
                })
            }
        }

        #[derive(Debug, Clone)]
        struct CPr(CreateProject<'static>);

        impl Arbitrary for CPr {
            fn arbitrary(g: &mut Gen) -> Self {
                CPr(CreateProject {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    name: String::arbitrary(g).into(),
                    services: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                    users: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                })
            }
        }

        quickcheck! {
            fn project(o: Pr) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn projects(o: Vec<Pr>) -> TestResult {
                let empty: Vec<Project> = vec![];
                let cbor = minicbor::to_vec(&empty).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Project> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(&o).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_project(o: CPr) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_project", SCHEMA, &cbor) {
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
            // Create node manager to handle requests
            let route = NodeManager::test_create(ctx, "projects", ProjectServer::default()).await?;
            let cloud_route = route_to_multiaddr(&route!["cloud"]).unwrap();

            let s_id = "space-id";

            // Create project
            let req = CreateProject::new("p1", &[], &["service"]);
            let mut buf = vec![];
            Request::builder(Method::Post, format!("v0/projects/{s_id}"))
                .body(CloudRequestWrapper::new(req, &cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let p = dec.decode::<Project>()?;
            assert_eq!(&p.name, "p1");
            assert_eq!(&p.services, &["service"]);
            let p_id = p.id.to_string();
            let p_name = p.name.to_string();

            // Retrieve it
            let mut buf = vec![];
            Request::builder(Method::Get, format!("v0/projects/{p_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let p = dec.decode::<Project>()?;
            assert_eq!(&p.id, &p_id);

            // Retrieve it by name
            let mut buf = vec![];
            Request::builder(Method::Get, format!("v0/projects/{s_id}/{p_name}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let p = dec.decode::<Project>()?;
            assert_eq!(&p.id, &p_id);

            // List it
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/projects")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let list = dec.decode::<Vec<Project>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &p_id);

            // Remove it
            let mut buf = vec![];
            Request::builder(Method::Delete, format!("v0/projects/{s_id}/{p_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));

            // Check list returns empty vec
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/projects")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status(), Some(Status::Ok));
            let list = dec.decode::<Vec<Project>>()?;
            assert!(list.is_empty());

            // Check that retrieving it returns a not found error
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/projects/{p_id}")
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
    pub struct ProjectServer(HashMap<String, Project<'static>>);

    #[ockam_core::worker]
    impl Worker for ProjectServer {
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

    impl ProjectServer {
        fn on_request(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            let r = match req.method() {
                Some(Method::Get) => match req.path_segments::<4>().as_slice() {
                    // Get all projects:
                    [_] => Response::ok(req.id())
                        .body(encode::ArrayIter::new(self.0.values()))
                        .to_vec()?,
                    // Get a single project:
                    [_, id] => {
                        if let Some(n) = self.0.get(*id) {
                            Response::ok(req.id()).body(n).to_vec()?
                        } else {
                            Response::not_found(req.id()).to_vec()?
                        }
                    }
                    // Get a single project by name:
                    [_, _, name] => {
                        if let Some((_, n)) = self.0.iter().find(|(_, n)| n.name == *name) {
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
                    if let Ok(project) = dec.decode::<CreateProject>() {
                        let obj = Project {
                            #[cfg(feature = "tag")]
                            tag: TypeTag,
                            id: u32::arbitrary(&mut rng).to_string().into(),
                            name: project.name.to_string().into(),
                            space_name: String::arbitrary(&mut rng).into(),
                            services: project
                                .services
                                .iter()
                                .map(|x| x.to_string().into())
                                .collect(),
                            access_route: String::arbitrary(&mut rng).into(),
                            users: project.users.iter().map(|x| x.to_string().into()).collect(),
                            space_id: "space-id".into(),
                            identity: Some(String::arbitrary(&mut rng).into()),
                        };
                        self.0.insert(obj.id.to_string(), obj.clone());
                        Response::ok(req.id()).body(&obj).to_vec()?
                    } else {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                }
                Some(Method::Delete) => match req.path_segments::<4>().as_slice() {
                    [_, _, id] => {
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
                    error!("{req:?}");
                    Response::bad_request(req.id()).to_vec()?
                }
            };
            Ok(r)
        }
    }
}
