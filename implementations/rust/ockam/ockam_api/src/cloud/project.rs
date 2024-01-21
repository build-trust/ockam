use miette::{miette, IntoDiagnostic};
use std::str::FromStr;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam::identity::models::ChangeHistory;
use ockam::identity::{Identifier, Identity};
use ockam_core::api::Request;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::{tokio, Context};

use crate::cloud::addon::ConfluentConfig;
use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;
use crate::cloud::operation::{Operation, Operations};
use crate::cloud::share::ShareScope;
use crate::cloud::{ControllerClient, ORCHESTRATOR_AWAIT_TIMEOUT};
use crate::error::ApiError;
use crate::minicbor_url::Url;
use crate::nodes::InMemoryNode;

use super::share::RoleInShare;

const TARGET: &str = "ockam_api::cloud::project";

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
    pub users: Vec<EmailAddress>,

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
    pub confluent_config: Option<ConfluentConfig>,

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
    #[n(1)] pub email: EmailAddress,
    #[n(2)] pub id: u64,
    #[n(3)] pub role: RoleInShare,
    #[n(4)] pub scope: ShareScope,
}

impl Project {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn identifier(&self) -> Result<Identifier> {
        match &self.identity.clone() {
            Some(identifier) => Ok(identifier.clone()),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no identity has been created for the project {}", self.name),
            )),
        }
    }

    pub fn project_name(&self) -> String {
        self.name.clone()
    }

    pub fn access_route(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(&self.access_route).map_err(|e| ApiError::core(e.to_string()))
    }

    pub fn authority_access_route(&self) -> Result<MultiAddr> {
        match &self.authority_access_route {
            Some(authority_access_route) => MultiAddr::from_str(authority_access_route)
                .map_err(|e| ApiError::core(e.to_string())),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!(
                    "no authority access route has been configured for the project {}",
                    self.name
                ),
            )),
        }
    }

    /// Return the decoded authority change history
    /// This method does not verify the change history so it does not require to be async
    pub fn authority_change_history(&self) -> Result<ChangeHistory> {
        match &self.authority_identity {
            Some(authority_identity) => {
                let decoded = hex::decode(authority_identity.as_bytes())
                    .map_err(|e| Error::new(Origin::Api, Kind::NotFound, e.to_string()))?;
                Ok(ChangeHistory::import(&decoded)?)
            }
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!(
                    "no authority change history has been configured for the project {}",
                    self.name
                ),
            )),
        }
    }

    /// Return the identifier of the project's authority
    pub async fn authority_identifier(&self) -> Result<Identifier> {
        Ok(self.authority_identity().await?.identifier().clone())
    }

    /// Return the identity of the project's authority
    pub async fn authority_identity(&self) -> Result<Identity> {
        match &self.authority_identity {
            Some(authority_identity) => Ok(Identity::create(authority_identity)
                .await
                .map_err(|e| Error::new(Origin::Api, Kind::Serialization, e.to_string()))?),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!(
                    "no authority identity has been configured for the project {}",
                    self.name
                ),
            )),
        }
    }

    pub fn is_admin(&self, user: &UserInfo) -> bool {
        self.user_roles
            .iter()
            .any(|ur| ur.role == RoleInShare::Admin && ur.email == user.email)
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
pub struct OrchestratorVersionInfo {
    /// The version of the Orchestrator Controller
    #[cbor(n(1))]
    pub version: Option<String>,

    /// The version of the Projects
    #[cbor(n(2))]
    pub project_version: Option<String>,
}

impl OrchestratorVersionInfo {
    pub fn version(&self) -> String {
        self.version.clone().unwrap_or("N/A".to_string())
    }

    pub fn project_version(&self) -> String {
        self.project_version.clone().unwrap_or("N/A".to_string())
    }
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

#[async_trait]
pub trait Projects {
    async fn create_project(
        &self,
        ctx: &Context,
        space_id: &str,
        name: &str,
        users: Vec<String>,
    ) -> miette::Result<Project>;

    async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project>;

    async fn get_project_by_name(
        &self,
        ctx: &Context,
        project_name: &str,
    ) -> miette::Result<Project>;

    async fn get_project_by_name_or_default(
        &self,
        ctx: &Context,
        project_name: &Option<String>,
    ) -> miette::Result<Project>;

    async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()>;

    async fn delete_project_by_name(
        &self,
        ctx: &Context,
        space_name: &str,
        project_name: &str,
    ) -> miette::Result<()>;

    async fn get_orchestrator_version_info(
        &self,
        ctx: &Context,
    ) -> miette::Result<OrchestratorVersionInfo>;

    async fn get_admin_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>>;

    async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project>;

    async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project>;
}

impl ControllerClient {
    pub async fn create_project(
        &self,
        ctx: &Context,
        space_id: &str,
        name: &str,
        users: Vec<String>,
    ) -> miette::Result<Project> {
        trace!(target: TARGET, %space_id, project_name = name, "creating project");
        let req = Request::post(format!("/v1/spaces/{space_id}/projects"))
            .body(CreateProject::new(name.to_string(), users));
        self.secure_client
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn get_project(&self, ctx: &Context, project_id: &str) -> miette::Result<Project> {
        trace!(target: TARGET, %project_id, "getting project");
        let req = Request::get(format!("/v0/{project_id}"));
        self.secure_client
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn delete_project(
        &self,
        ctx: &Context,
        space_id: &str,
        project_id: &str,
    ) -> miette::Result<()> {
        trace!(target: TARGET, %space_id, %project_id, "deleting project");
        let req = Request::delete(format!("/v0/{space_id}/{project_id}"));
        self.secure_client
            .tell(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn get_orchestrator_version_info(
        &self,
        ctx: &Context,
    ) -> miette::Result<OrchestratorVersionInfo> {
        trace!(target: TARGET, "getting orchestrator version information");
        self.secure_client
            .ask(ctx, "version_info", Request::get(""))
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn list_projects(&self, ctx: &Context) -> miette::Result<Vec<Project>> {
        let req = Request::get("/v0");
        self.secure_client
            .ask(ctx, "projects", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    pub async fn wait_until_project_creation_operation_is_complete(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        let operation_id = match &project.operation_id {
            Some(operation_id) => operation_id,
            // if no operation id is present this means that the operation is already complete
            None => return Ok(project),
        };

        let result = self
            .wait_until_operation_is_complete(ctx, operation_id)
            .await;
        match result {
            Ok(()) => self.get_project(ctx, &project.id).await,
            Err(e) => Err(miette!("The project creation did not complete: {:?}", e)),
        }
    }

    pub async fn wait_until_project_is_ready(
        &self,
        ctx: &Context,
        project: Project,
    ) -> miette::Result<Project> {
        let retry_strategy = FixedInterval::from_millis(5000)
            .take((ORCHESTRATOR_AWAIT_TIMEOUT.as_millis() / 5000) as usize);
        Retry::spawn(retry_strategy.clone(), || async {
            if let Ok(project) = self.get_project(ctx, &project.id).await {
                if project.is_ready() {
                    Ok(project)
                } else {
                    debug!("the project {} is not ready yet. Retrying...", &project.id);
                    Err(miette!(
                        "The project {} is not ready. Please try again.",
                        &project.id
                    ))
                }
            } else {
                Err(miette!(
                    "The project {} could not be retrieved",
                    &project.id
                ))
            }
        })
        .await
    }
}

#[async_trait]
impl Operations for InMemoryNode {
    async fn get_operation(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<Option<Operation>> {
        self.create_controller()
            .await?
            .get_operation(ctx, operation_id)
            .await
    }

    async fn wait_until_operation_is_complete(
        &self,
        ctx: &Context,
        operation_id: &str,
    ) -> miette::Result<()> {
        self.create_controller()
            .await?
            .wait_until_operation_is_complete(ctx, operation_id)
            .await
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

#[cfg(test)]
mod tests {
    use ockam::identity::models::IDENTIFIER_LEN;
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

    #[test]
    fn test_is_admin() {
        let mut g = Gen::new(100);
        let mut project = Project::arbitrary(&mut g);

        // it is possible to test if a user an administrator
        // of the project by comparing the user email and the project role email
        // the email comparison is case insensitive
        project.user_roles = vec![create_admin("test@ockam.io")];
        assert!(project.is_admin(&create_user("test@ockam.io")));
        assert!(project.is_admin(&create_user("tEst@ockam.io")));
        assert!(project.is_admin(&create_user("test@Ockam.io")));
        assert!(project.is_admin(&create_user("TEST@OCKAM.IO")));
    }

    /// HELPERS

    fn create_admin(email: &str) -> ProjectUserRole {
        ProjectUserRole {
            email: email.try_into().unwrap(),
            id: 1,
            role: RoleInShare::Admin,
            scope: ShareScope::Project,
        }
    }

    fn create_user(email: &str) -> UserInfo {
        UserInfo {
            sub: "name".to_string(),
            nickname: "nickname".to_string(),
            name: "name".to_string(),
            picture: "picture".to_string(),
            updated_at: "noon".to_string(),
            email: email.try_into().unwrap(),
            email_verified: false,
        }
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
                users: vec![EmailAddress::arbitrary(g), EmailAddress::arbitrary(g)],
                space_id: String::arbitrary(g),
                identity: bool::arbitrary(g).then_some(Identifier(identifier)),
                authority_access_route: bool::arbitrary(g).then(|| String::arbitrary(g)),
                authority_identity: bool::arbitrary(g)
                    .then(|| hex::encode(<Vec<u8>>::arbitrary(g))),
                okta_config: bool::arbitrary(g).then(|| OktaConfig::arbitrary(g)),
                confluent_config: bool::arbitrary(g).then(|| ConfluentConfig::arbitrary(g)),
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
