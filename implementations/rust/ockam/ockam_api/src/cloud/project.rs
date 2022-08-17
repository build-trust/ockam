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
    #[serde(skip)]
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
            services: services.iter().map(|x| CowStr::from(x.as_ref())).collect(),
            users: users.iter().map(|x| CowStr::from(x.as_ref())).collect(),
        }
    }
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct AddEnroller<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<7361445>,
    #[b(1)] pub identity_id: CowStr<'a>,
    #[b(2)] pub description: Option<CowStr<'a>>,
}

#[derive(Encode, Decode, Serialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Enroller<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[n(0)] pub tag: TypeTag<4277633>,
    #[b(1)] pub identity_id: CowStr<'a>,
    #[b(2)] pub description: Option<CowStr<'a>>,
    #[b(3)] pub added_by: CowStr<'a>,
    #[b(4)] pub created_at: CowStr<'a>,
}

impl<'a> AddEnroller<'a> {
    pub fn new<S: Into<CowStr<'a>>>(identity_id: S, description: Option<S>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id: identity_id.into(),

            description: description.map(|s| s.into()),
        }
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::{Request, Response, Status};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeManager;

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManager {
        pub(crate) async fn create_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            space_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateProject> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "create_project";
            trace!(target: TARGET, %space_id, project_name = %req_body.name, "creating project");

            let req_builder = Request::post(format!("/v0/{space_id}")).body(req_body);
            self.request_controller(
                ctx,
                label,
                "create_project",
                cloud_route,
                "projects",
                req_builder,
            )
            .await
        }

        pub(crate) async fn list_projects(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_projects";
            trace!(target: TARGET, "listing projects");

            let req_builder = Request::get("/v0");
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
        }

        pub(crate) async fn get_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "get_project";
            trace!(target: TARGET, %project_id, "getting project");

            let req_builder = Request::get(format!("/v0/{project_id}"));
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
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

            let req_builder = Request::get("/v0");

            match self
                .request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
            {
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
            }
        }

        pub(crate) async fn delete_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "delete_project";
            trace!(target: TARGET, %space_id, %project_id, "deleting project");

            let req_builder = Request::delete(format!("/v0/{space_id}/{project_id}"));
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
        }

        pub(crate) async fn add_project_enroller(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<AddEnroller> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "add_enroller";
            trace!(target: TARGET, %project_id, "adding enroller");

            let req_builder = Request::post(format!("/v0/{project_id}/enrollers")).body(req_body);
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
        }

        pub(crate) async fn list_project_enrollers(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_enrollers";
            trace!(target: TARGET, %project_id, "listing enrollers");

            let req_builder = Request::get(format!("/v0/{project_id}/enrollers"));
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
        }

        pub(crate) async fn delete_project_enroller(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
            enroller_identity_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "delete_enroller";
            trace!(target: TARGET, %project_id, %enroller_identity_id, "deleting enroller");

            let req_builder =
                Request::delete(format!("/v0/{project_id}/enrollers/{enroller_identity_id}"));
            self.request_controller(ctx, label, None, cloud_route, "projects", req_builder)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use ockam_core::api::SCHEMA;

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
}
