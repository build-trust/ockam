use crate::cloud::addon::KafkaConfig;
use crate::cloud::email_address::EmailAddress;
use crate::cloud::share::{RoleInShare, ShareScope};
use crate::minicbor_url::Url;
use minicbor::{CborLen, Decode, Encode};
use ockam::identity::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq)]
#[cbor(map)]
pub struct ProjectModel {
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
    pub kafka_config: Option<KafkaConfig>,

    #[cbor(n(13))]
    pub version: Option<String>,

    #[cbor(n(14))]
    pub running: Option<bool>,

    #[cbor(n(15))]
    pub operation_id: Option<String>,

    #[cbor(n(16))]
    pub user_roles: Vec<ProjectUserRole>,

    #[cbor(n(17))]
    pub project_change_history: Option<String>,
}

#[derive(Encode, Decode, CborLen, Debug)]
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

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug)]
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

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq, Decode, Encode, CborLen, Deserialize, Serialize)]
#[cbor(map)]
#[rustfmt::skip]
pub struct ProjectUserRole {
    #[n(1)] pub email: EmailAddress,
    #[n(2)] pub id: u64,
    #[n(3)] pub role: RoleInShare,
    #[n(4)] pub scope: ShareScope,
}

impl Display for ProjectUserRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectUserRole")
            .field("email", &self.email)
            .field("id", &self.id)
            .field("role", &self.role)
            .field("scope", &self.scope)
            .finish()
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

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};
    use std::str::FromStr;

    use crate::schema::tests::validate_with_schema;

    use super::*;

    quickcheck! {
        fn project(p: ProjectModel) -> TestResult {
            validate_with_schema("project", p)
        }

        fn projects(ps: Vec<ProjectModel>) -> TestResult {
            validate_with_schema("projects", ps)
        }

        fn create_project(cp: CreateProject) -> TestResult {
            validate_with_schema("create_project", cp)
        }
    }

    /// HELPERS

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

    impl Arbitrary for ProjectModel {
        fn arbitrary(g: &mut Gen) -> Self {
            let identifier = Identifier::from_str(
                "I923829d0397a06fa862be5a87b7966959b8ef99ab6455b843ca9131a747b4819",
            )
            .unwrap();
            let change_history = "81825837830101583285f68200815820f405e06d988fa8039cce1cd0ae607e46847c1b64bc459ca9d89dd9b21ae30681f41a654cebe91a7818eee98200815840494c9b70e8a9ad5593fceb478f722a513b4bd39fa70f4265d584253bc24617d0eb498ce532273f6d0d5326921e013696fce57c20cc6c4008f74b816810f0b009".to_string();

            ProjectModel {
                id: String::arbitrary(g),
                name: String::arbitrary(g),
                space_name: String::arbitrary(g),
                access_route: String::arbitrary(g),
                users: vec![EmailAddress::arbitrary(g), EmailAddress::arbitrary(g)],
                space_id: String::arbitrary(g),
                identity: bool::arbitrary(g).then_some(identifier),
                project_change_history: bool::arbitrary(g).then_some(change_history.clone()),
                authority_access_route: bool::arbitrary(g).then(|| String::arbitrary(g)),
                authority_identity: Some(change_history.clone()),
                okta_config: bool::arbitrary(g).then(|| OktaConfig::arbitrary(g)),
                kafka_config: bool::arbitrary(g).then(|| KafkaConfig::arbitrary(g)),
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
