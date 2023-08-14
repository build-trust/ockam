use std::str::FromStr;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam::identity::Identifier;
use ockam_core::Result;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio;

use crate::cloud::addon::ConfluentConfigResponse;
use crate::error::ApiError;
use crate::minicbor_url::Url;

use super::share::RoleInShare;
use super::ProjectUserRole;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct Project {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<9056532>,

    #[cbor(n(1))]
    pub id: String,

    #[cbor(n(2))]
    pub name: String,

    #[cbor(n(3))]
    pub space_name: String,

    #[cbor(n(5))]
    pub access_route: String,

    #[cbor(n(6))]
    pub users: Vec<String>,

    #[cbor(n(7))]
    pub space_id: String,

    #[cbor(n(8))]
    pub identity: Option<Identifier>,

    #[cbor(n(9))]
    pub authority_access_route: Option<String>,

    #[cbor(n(10))]
    pub authority_identity: Option<String>,

    #[cbor(n(11))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub okta_config: Option<OktaConfig>,

    #[cbor(n(12))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confluent_config: Option<ConfluentConfigResponse>,

    #[cbor(n(13))]
    pub version: Option<String>,

    #[cbor(n(14))]
    pub running: Option<bool>,

    #[cbor(n(15))]
    pub operation_id: Option<String>,

    #[cbor(n(16))]
    pub user_roles: Vec<ProjectUserRole>,
}

impl Project {
    pub fn access_route(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(&self.access_route).map_err(|e| ApiError::generic(&e.to_string()))
    }

    pub fn has_admin_with_email(&self, email: &str) -> bool {
        self.user_roles
            .iter()
            .any(|ur| ur.role == RoleInShare::Admin && ur.email == email)
    }

    pub async fn is_reachable(&self) -> Result<bool> {
        let socket_addr = self.access_route_socket_addr()?;
        Ok(tokio::net::TcpStream::connect(&socket_addr).await.is_ok())
    }

    pub fn is_ready(&self) -> bool {
        !(self.access_route.is_empty()
            || self.authority_access_route.is_none()
            || self.identity.is_none()
            || self.authority_identity.is_none())
    }

    // Converts the `access_route` MultiAddr into a single Address, which will
    // return the host and port of the project node.
    // Ex: if access_route is "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api",
    // then this will return the string "node.dnsaddr.com:4000".
    fn access_route_socket_addr(&self) -> Result<String> {
        let ma = self.access_route()?;
        ma.to_socket_addr()
            .map_err(|e| ApiError::generic(&e.to_string()))
    }
}

#[derive(Decode, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct ProjectVersion {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<9116532>,

    /// The version of the Orchestrator Controller
    #[cbor(n(1))]
    pub version: Option<String>,

    /// The version of the Projects
    #[cbor(n(2))]
    pub project_version: Option<String>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OktaConfig {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<6434814>,
    #[cbor(n(1))] pub tenant_base_url: Url,
    #[cbor(n(2))] pub certificate: String,
    #[cbor(n(3))] pub client_id: String,
    #[cbor(n(4))] pub attributes: Vec<String>,
}

impl OktaConfig {
    pub fn new<S: ToString>(
        tenant_base_url: Url,
        certificate: S,
        client_id: S,
        attributes: Vec<String>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant_base_url,
            certificate: certificate.to_string(),
            client_id: client_id.to_string(),
            attributes,
        }
    }

    pub fn new_empty_attributes<S: ToString>(
        tenant_base_url: Url,
        certificate: S,
        client_id: S,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant_base_url,
            certificate: certificate.to_string(),
            client_id: client_id.to_string(),
            attributes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OktaAuth0 {
    pub tenant_base_url: Url,
    pub client_id: String,
    pub certificate: String,
}

impl From<OktaConfig> for OktaAuth0 {
    fn from(c: OktaConfig) -> Self {
        Self {
            tenant_base_url: c.tenant_base_url,
            client_id: c.client_id,
            certificate: c.certificate,
        }
    }
}

impl From<OktaAuth0> for OktaConfig {
    fn from(val: OktaAuth0) -> Self {
        OktaConfig::new_empty_attributes(val.tenant_base_url, val.certificate, val.client_id)
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InfluxDBTokenLeaseManagerConfig {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<4166488>,
    #[cbor(n(1))] pub endpoint: String,
    #[cbor(n(2))] pub token: String,
    #[cbor(n(3))] pub org_id: String,
    #[cbor(n(4))] pub permissions: String,
    #[cbor(n(5))] pub max_ttl_secs: i32,
    #[cbor(n(6))] pub user_access_rule: Option<String>,
    #[cbor(n(7))] pub admin_access_rule: Option<String>,
}

impl InfluxDBTokenLeaseManagerConfig {
    pub fn new<S: Into<String>>(
        endpoint: S,
        token: S,
        org_id: S,
        permissions: S,
        max_ttl_secs: i32,
        user_access_rule: Option<S>,
        admin_access_rule: Option<S>,
    ) -> Self {
        let uar: Option<String> = user_access_rule.map(|s| s.into());

        let aar: Option<String> = admin_access_rule.map(|s| s.into());

        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            endpoint: endpoint.into(),
            token: token.into(),
            org_id: org_id.into(),
            permissions: permissions.into(),
            max_ttl_secs,
            user_access_rule: uar,
            admin_access_rule: aar,
        }
    }
}

#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateProject {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8669570>,
    #[n(1)] pub name: String,
    #[n(3)] pub users: Vec<String>,
}

impl CreateProject {
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
    use tokio_retry::strategy::FixedInterval;
    use tokio_retry::Retry;
    use tracing::trace;

    use ockam_core::api::{Request, Response};
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::operation::Operation;
    use crate::cloud::{
        BareCloudRequestWrapper, CloudRequestWrapper, ORCHESTRATOR_AWAIT_TIMEOUT_MS,
    };
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManager {
        pub async fn create_project(
            &self,
            ctx: &Context,
            route: &MultiAddr,
            space_id: &str,
            project_name: &str,
            users: Vec<String>,
        ) -> Result<Project> {
            let request = CloudRequestWrapper::new(
                CreateProject::new(project_name.to_string(), users),
                route,
                None,
            );
            Response::parse_response_body(
                self.create_project_response(ctx, request, space_id)
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn create_project_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateProject>,
            space_id: &str,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;
            let label = "create_project";
            trace!(target: TARGET, %space_id, project_name = %req_body.name, "creating project");
            let req_builder =
                Request::post(format!("/v1/spaces/{space_id}/projects")).body(&req_body);

            self.request_controller(
                ctx,
                label,
                "create_project",
                &cloud_multiaddr,
                "projects",
                req_builder,
                req_wrapper.identity_name,
            )
            .await
        }

        pub async fn list_projects(
            &self,
            ctx: &Context,
            route: &MultiAddr,
        ) -> Result<Vec<Project>> {
            let bytes = self
                .list_projects_response(ctx, CloudRequestWrapper::bare(route))
                .await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub(crate) async fn list_projects_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let label = "list_projects";
            trace!(target: TARGET, "listing projects");
            let req_builder = Request::get("/v0");

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                "projects",
                req_builder,
                None,
            )
            .await
        }

        pub async fn get_project(
            &self,
            ctx: &Context,
            route: &MultiAddr,
            project_id: &str,
        ) -> Result<Project> {
            Response::parse_response_body(
                self.get_project_response(ctx, CloudRequestWrapper::bare(route), project_id)
                    .await?
                    .as_slice(),
            )
        }

        pub async fn wait_until_project_is_ready(
            &self,
            ctx: &Context,
            route: &MultiAddr,
            project: Project,
        ) -> Result<Project> {
            if project.is_ready() {
                return Ok(project);
            }
            let operation_id = match &project.operation_id {
                Some(operation_id) => operation_id,
                None => {
                    return Err(ApiError::generic("Project has no operation id"));
                }
            };
            let retry_strategy =
                FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);
            let operation = Retry::spawn(retry_strategy.clone(), || async {
                if let Ok(res) = self.get_operation(ctx, route, operation_id).await {
                    if let Ok(operation) =
                        Response::parse_response_body::<Operation>(res.as_slice())
                    {
                        if operation.is_completed() {
                            return Ok(operation);
                        }
                    }
                }
                Err(ApiError::generic(
                    "Project is not reachable yet. Retrying...",
                ))
            })
            .await?;
            if operation.is_successful() {
                self.get_project(ctx, route, &project.id).await
            } else {
                Err(ApiError::generic("Operation failed. Please try again."))
            }
        }

        pub(crate) async fn get_project_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "get_project";
            trace!(target: TARGET, %project_id, "getting project");
            let req_builder = Request::get(format!("/v0/{project_id}"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                "projects",
                req_builder,
                None,
            )
            .await
        }

        pub async fn get_project_version(
            &self,
            ctx: &Context,
            route: &MultiAddr,
        ) -> Result<ProjectVersion> {
            Response::parse_response_body(
                self.get_project_version_response(ctx, CloudRequestWrapper::bare(route))
                    .await?
                    .as_slice(),
            )
        }

        pub(crate) async fn get_project_version_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "version_info";
            trace!(target: TARGET, "getting project version");
            let req_builder = Request::get("");

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                "version_info",
                req_builder,
                None,
            )
            .await
        }

        pub async fn delete_project(
            &self,
            ctx: &Context,
            route: &MultiAddr,
            space_id: &str,
            project_id: &str,
        ) -> Result<()> {
            let _ = self
                .delete_project_response(
                    ctx,
                    CloudRequestWrapper::bare(route),
                    space_id,
                    project_id,
                )
                .await?;
            Ok(())
        }

        pub(crate) async fn delete_project_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let cloud_multiaddr = req_wrapper.multiaddr()?;

            let label = "delete_project";
            trace!(target: TARGET, %space_id, %project_id, "deleting project");

            let req_builder = Request::delete(format!("/v0/{space_id}/{project_id}"));

            self.request_controller(
                ctx,
                label,
                None,
                &cloud_multiaddr,
                "projects",
                req_builder,
                None,
            )
            .await
        }
    }

    impl NodeManagerWorker {
        pub(crate) async fn create_project_response(
            &self,
            ctx: &Context,
            req_wrapper: CloudRequestWrapper<CreateProject>,
            space_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .create_project_response(ctx, req_wrapper, space_id)
                .await
        }

        pub async fn list_projects(
            &self,
            ctx: &Context,
            route: &MultiAddr,
        ) -> Result<Vec<Project>> {
            let node_manager = self.inner().read().await;
            node_manager.list_projects(ctx, route).await
        }

        pub(crate) async fn list_projects_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.list_projects_response(ctx, req_wrapper).await
        }

        pub(crate) async fn get_project_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .get_project_response(ctx, req_wrapper, project_id)
                .await
        }

        pub(crate) async fn get_project_version_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .get_project_version_response(ctx, req_wrapper)
                .await
        }

        pub(crate) async fn delete_project_response(
            &self,
            ctx: &Context,
            req_wrapper: BareCloudRequestWrapper,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .delete_project_response(ctx, req_wrapper, space_id, project_id)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use ockam::identity::models::IDENTIFIER_LEN;
    use quickcheck::{Arbitrary, Gen};

    use super::*;

    impl Arbitrary for OktaConfig {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: Default::default(),
                tenant_base_url: Url::new(url::Url::parse("http://example.com/").unwrap()),
                certificate: String::arbitrary(g),
                client_id: String::arbitrary(g),
                attributes: Vec::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Project {
        fn arbitrary(g: &mut Gen) -> Self {
            let identifier = [0u8; IDENTIFIER_LEN];
            identifier.map(|_| <u8>::arbitrary(g));

            Project {
                #[cfg(feature = "tag")]
                tag: Default::default(),
                id: String::arbitrary(g),
                name: String::arbitrary(g),
                space_name: String::arbitrary(g),
                access_route: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
                space_id: String::arbitrary(g),
                identity: bool::arbitrary(g).then(|| Identifier::new(identifier)),
                authority_access_route: bool::arbitrary(g).then(|| String::arbitrary(g)),
                authority_identity: bool::arbitrary(g)
                    .then(|| hex::encode(<Vec<u8>>::arbitrary(g))),
                okta_config: bool::arbitrary(g).then(|| OktaConfig::arbitrary(g)),
                confluent_config: bool::arbitrary(g).then(|| ConfluentConfigResponse::arbitrary(g)),
                version: Some(String::arbitrary(g)),
                running: bool::arbitrary(g).then(|| bool::arbitrary(g)),
                operation_id: bool::arbitrary(g).then(|| String::arbitrary(g)),
                user_roles: vec![],
            }
        }
    }

    impl Arbitrary for CreateProject {
        fn arbitrary(g: &mut Gen) -> Self {
            CreateProject {
                #[cfg(feature = "tag")]
                tag: Default::default(),
                name: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
            }
        }
    }

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::schema::SCHEMA;

        use super::*;

        quickcheck! {
            fn project(o: Project) -> TestResult {
                let cbor = minicbor::to_vec(o).unwrap();
                if let Err(e) = validate_cbor_bytes("project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn projects(o: Vec<Project>) -> TestResult {
                let empty: Vec<Project> = vec![];
                let cbor = minicbor::to_vec(empty).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let cbor = minicbor::to_vec(o).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_project(o: CreateProject) -> TestResult {
                let cbor = minicbor::to_vec(o).unwrap();
                if let Err(e) = validate_cbor_bytes("create_project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }

    #[test]
    fn convert_access_route_to_socket_addr() {
        let mut g = Gen::new(100);
        let mut p = Project::arbitrary(&mut g);
        p.access_route = "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api".into();

        let socket_addr = p.access_route_socket_addr().unwrap();
        assert_eq!(&socket_addr, "node.dnsaddr.com:4000");
    }
}
