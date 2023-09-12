use std::str::FromStr;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use ockam::identity::Identifier;
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio;

use crate::cloud::addon::ConfluentConfigResponse;
use crate::cloud::share::ShareScope;
use crate::error::ApiError;
use crate::minicbor_url::Url;

use super::share::RoleInShare;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct Project {
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

#[derive(Clone, Debug, Eq, PartialEq, Decode, Deserialize, Encode, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct ProjectUserRole {
    #[n(1)] pub email: String,
    #[n(2)] pub id: usize,
    #[n(3)] pub role: RoleInShare,
    #[n(4)] pub scope: ShareScope,
}

impl Project {
    pub fn access_route(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(&self.access_route).map_err(|e| ApiError::core(e.to_string()))
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
            .map_err(|e| ApiError::core(e.to_string()))
    }
}

#[derive(Decode, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct ProjectVersion {
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
    #[n(1)] pub name: String,
    #[n(3)] pub users: Vec<String>,
}

impl CreateProject {
    pub fn new(name: String, users: Vec<String>) -> Self {
        Self { name, users }
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
    use crate::cloud::{CloudRequestWrapper, ORCHESTRATOR_AWAIT_TIMEOUT_MS};
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManager {
        pub async fn create_project(
            &self,
            ctx: &Context,
            space_id: &str,
            project_name: &str,
            users: Vec<String>,
        ) -> Result<Project> {
            let request =
                CloudRequestWrapper::new(CreateProject::new(project_name.to_string(), users), None);
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
            let req_body = req_wrapper.req;
            trace!(target: TARGET, %space_id, project_name = %req_body.name, "creating project");
            let req = Request::post(format!("/v1/spaces/{space_id}/projects")).body(&req_body);

            self.request_controller(ctx, "projects", req, req_wrapper.identity_name)
                .await
        }

        pub async fn list_projects(&self, ctx: &Context) -> Result<Vec<Project>> {
            let bytes = self.list_projects_response(ctx).await?;
            Response::parse_response_body(bytes.as_slice())
        }

        pub(crate) async fn list_projects_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            trace!(target: TARGET, "listing projects");
            let req = Request::get("/v0");

            self.request_controller(ctx, "projects", req, None).await
        }

        pub async fn get_project(&self, ctx: &Context, project_id: &str) -> Result<Project> {
            Response::parse_response_body(
                self.get_project_response(ctx, project_id).await?.as_slice(),
            )
        }

        pub async fn wait_until_project_is_ready(
            &self,
            ctx: &Context,
            project: Project,
        ) -> Result<Project> {
            if project.is_ready() {
                return Ok(project);
            }
            let operation_id = match &project.operation_id {
                Some(operation_id) => operation_id,
                None => {
                    return Err(ApiError::core("Project has no operation id"));
                }
            };
            let retry_strategy =
                FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);
            let operation = Retry::spawn(retry_strategy.clone(), || async {
                if let Ok(res) = self.get_operation(ctx, operation_id).await {
                    if let Ok(operation) =
                        Response::parse_response_body::<Operation>(res.as_slice())
                    {
                        if operation.is_completed() {
                            return Ok(operation);
                        }
                    }
                }
                Err(ApiError::core("Project is not reachable yet. Retrying..."))
            })
            .await?;
            if operation.is_successful() {
                self.get_project(ctx, &project.id).await
            } else {
                Err(ApiError::core("Operation failed. Please try again."))
            }
        }

        pub(crate) async fn get_project_response(
            &self,
            ctx: &Context,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, %project_id, "getting project");
            let req = Request::get(format!("/v0/{project_id}"));

            self.request_controller(ctx, "projects", req, None).await
        }

        pub async fn get_project_version(&self, ctx: &Context) -> Result<ProjectVersion> {
            Response::parse_response_body(self.get_project_version_response(ctx).await?.as_slice())
        }

        pub(crate) async fn get_project_version_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            trace!(target: TARGET, "getting project version");
            let req = Request::get("");

            self.request_controller(ctx, "version_info", req, None)
                .await
        }

        pub async fn delete_project(
            &self,
            ctx: &Context,
            space_id: &str,
            project_id: &str,
        ) -> Result<()> {
            let _ = self
                .delete_project_response(ctx, space_id, project_id)
                .await?;
            Ok(())
        }

        pub(crate) async fn delete_project_response(
            &self,
            ctx: &Context,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            trace!(target: TARGET, %space_id, %project_id, "deleting project");
            let req = Request::delete(format!("/v0/{space_id}/{project_id}"));

            self.request_controller(ctx, "projects", req, None).await
        }
    }
`
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

        pub async fn list_projects(&self, ctx: &Context) -> Result<Vec<Project>> {
            let node_manager = self.inner().read().await;
            node_manager.list_projects(ctx).await
        }

        pub(crate) async fn list_projects_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.list_projects_response(ctx).await
        }

        pub(crate) async fn get_project_response(
            &self,
            ctx: &Context,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.get_project_response(ctx, project_id).await
        }

        pub(crate) async fn get_project_version_response(&self, ctx: &Context) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager.get_project_version_response(ctx).await
        }

        pub(crate) async fn delete_project_response(
            &self,
            ctx: &Context,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let node_manager = self.inner().read().await;
            node_manager
                .delete_project_response(ctx, space_id, project_id)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use ockam::identity::models::IDENTIFIER_LEN;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use crate::schema::tests::validate_with_schema;

    use super::*;

    quickcheck! {
        fn project(p: Project) -> TestResult {
            validate_with_schema("project", p)
        }

        fn projects(ps: Vec<Project>) -> TestResult {
            validate_with_schema("projects", ps)
        }

        fn create_project(cp: CreateProject) -> TestResult {
            validate_with_schema("create_project", cp)
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

    impl Arbitrary for OktaConfig {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
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
                id: String::arbitrary(g),
                name: String::arbitrary(g),
                space_name: String::arbitrary(g),
                access_route: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
                space_id: String::arbitrary(g),
                identity: bool::arbitrary(g).then_some(Identifier(identifier)),
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
                name: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
            }
        }
    }
}
