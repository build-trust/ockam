use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::{CredentialRetrieverCreator, Identifier, RemoteCredentialRetrieverInfo};
use ockam_core::errcode::{Kind, Origin};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

pub const PROJECT_MEMBER_SCOPE_PREFIX: &str = "project-member-";
pub const PROJECT_ADMIN_SCOPE_PREFIX: &str = "project-admin-";
pub const ACCOUNT_ADMIN_SCOPE_PREFIX: &str = "account-admin-";

#[derive(Clone)]
pub struct CredentialRetrieverCreators {
    pub(crate) project_member: Option<Arc<dyn CredentialRetrieverCreator>>,
    pub(crate) project_admin: Option<Arc<dyn CredentialRetrieverCreator>>,
    pub(crate) _account_admin: Option<Arc<dyn CredentialRetrieverCreator>>,
}

#[derive(Debug)]
pub enum CredentialScope {
    ProjectMember { project_id: String },
    ProjectAdmin { project_id: String },
    AccountAdmin { account_id: String },
}

impl FromStr for CredentialScope {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(project_id) = s.strip_prefix(PROJECT_MEMBER_SCOPE_PREFIX) {
            return Ok(CredentialScope::ProjectMember {
                project_id: project_id.to_string(),
            });
        }

        if let Some(project_id) = s.strip_prefix(PROJECT_ADMIN_SCOPE_PREFIX) {
            return Ok(CredentialScope::ProjectAdmin {
                project_id: project_id.to_string(),
            });
        }

        if let Some(account_id) = s.strip_prefix(ACCOUNT_ADMIN_SCOPE_PREFIX) {
            return Ok(CredentialScope::AccountAdmin {
                account_id: account_id.to_string(),
            });
        }

        Err(ockam_core::Error::new(
            Origin::Api,
            Kind::Invalid,
            "Invalid credential scope format",
        ))
    }
}

impl Display for CredentialScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            CredentialScope::ProjectMember { project_id } => {
                format!("{}{}", PROJECT_MEMBER_SCOPE_PREFIX, project_id)
            }
            CredentialScope::ProjectAdmin { project_id } => {
                format!("{}{}", PROJECT_ADMIN_SCOPE_PREFIX, project_id)
            }
            CredentialScope::AccountAdmin { account_id } => {
                format!("{}{}", ACCOUNT_ADMIN_SCOPE_PREFIX, account_id)
            }
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug)]
pub enum NodeManagerCredentialRetrieverOptions {
    None,
    CacheOnly {
        issuer: Identifier,
        scope: String,
    },
    Remote {
        info: RemoteCredentialRetrieverInfo,
        scope: String,
    },
    InMemory(CredentialAndPurposeKey),
}

pub struct NodeManagerTrustOptions {
    pub(super) project_member_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
    pub(super) project_authority: Option<Identifier>,
    pub(super) project_admin_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
    pub(super) _account_admin_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
}

impl NodeManagerTrustOptions {
    pub fn new(
        project_member_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
        project_admin_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
        project_authority: Option<Identifier>,
        account_admin_credential_retriever_options: NodeManagerCredentialRetrieverOptions,
    ) -> Self {
        Self {
            project_member_credential_retriever_options,
            project_admin_credential_retriever_options,
            project_authority,
            _account_admin_credential_retriever_options: account_admin_credential_retriever_options,
        }
    }
}
