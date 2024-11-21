use serde::Serialize;
use std::fmt::Write;
use std::str::FromStr;

use crate::cloud::enroll::auth0::UserInfo;
use crate::cloud::project::models::ProjectModel;
use crate::cloud::share::RoleInShare;
use crate::colors::color_primary;
use crate::error::ApiError;
use crate::output::Output;
use crate::terminal::fmt;

use crate::TransportRouteResolver;
use ockam::identity::{Identifier, Identity, Vault};
use ockam_core::compat::collections::HashSet;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio;

pub(super) const TARGET: &str = "ockam_api::cloud::project";

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    #[serde(flatten)]
    model: ProjectModel,
    #[serde(skip)]
    project_identity: Option<Identity>,
    #[serde(skip)]
    project_multiaddr: Option<MultiAddr>,
    #[serde(rename = "access_address")]
    project_socket_addr: Option<String>,
    #[serde(skip)]
    authority_identity: Option<Identity>,
    #[serde(skip)]
    authority_multiaddr: Option<MultiAddr>,
    #[serde(rename = "authority_access_address")]
    authority_socket_addr: Option<String>,
    egress_allow_list: Vec<String>,
}

impl Project {
    pub async fn import(model: ProjectModel) -> Result<Self> {
        let project_identity = match &model.project_change_history {
            Some(project_change_history) => Some(
                Identity::import_from_string(
                    model.identity.as_ref(),
                    project_change_history.as_str(),
                    Vault::create_verifying_vault(),
                )
                .await?,
            ),
            None => None,
        };

        let mut egress_allow_list = HashSet::new();
        let project_socket_addr;
        let project_multiaddr;
        if model.access_route.is_empty() {
            project_socket_addr = None;
            project_multiaddr = None;
        } else {
            let multiaddr = MultiAddr::from_str(&model.access_route)
                .map_err(|e| ApiError::core(e.to_string()))?;

            // Converts the `access_route` MultiAddr into a single Address, which will
            // return the host and port of the project node.
            // Ex: if access_route is "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api",
            // then this will return the string "node.dnsaddr.com:4000".
            let socket_addr = TransportRouteResolver::default()
                .allow_tcp()
                .socket_address(&multiaddr)?;
            project_socket_addr = Some(socket_addr.clone());
            egress_allow_list.insert(socket_addr);
            project_multiaddr = Some(multiaddr);
        }

        let authority_identity = match &model.authority_identity {
            Some(authority_change_history) => Some(
                Identity::import_from_string(
                    None,
                    authority_change_history.as_str(),
                    Vault::create_verifying_vault(),
                )
                .await?,
            ),
            None => None,
        };

        let authority_socket_addr;
        let authority_multiaddr;
        match &model.authority_access_route {
            Some(authority_access_route) => {
                let multiaddr = MultiAddr::from_str(authority_access_route)
                    .map_err(|e| ApiError::core(e.to_string()))?;
                let socket_addr = TransportRouteResolver::default()
                    .allow_tcp()
                    .socket_address(&multiaddr)?;
                authority_socket_addr = Some(socket_addr.clone());
                egress_allow_list.insert(socket_addr);
                authority_multiaddr = Some(multiaddr)
            }
            None => {
                authority_socket_addr = None;
                authority_multiaddr = None;
            }
        };

        let s = Self {
            model,
            project_identity,
            project_multiaddr,
            project_socket_addr,
            authority_identity,
            authority_multiaddr,
            authority_socket_addr,
            egress_allow_list: egress_allow_list.into_iter().collect(),
        };

        Ok(s)
    }

    pub fn model(&self) -> &ProjectModel {
        &self.model
    }

    pub fn name(&self) -> &str {
        self.model.name.as_str()
    }

    pub fn project_id(&self) -> &str {
        self.model.id.as_str()
    }

    /// Return the identity of the project's node
    pub fn project_identity(&self) -> Option<&Identity> {
        self.project_identity.as_ref()
    }

    pub fn project_identifier(&self) -> Option<Identifier> {
        self.model.identity.clone()
    }

    pub fn project_multiaddr(&self) -> Result<&MultiAddr> {
        match &self.project_multiaddr {
            Some(project_multiaddr) => Ok(project_multiaddr),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!(
                    "no project multiaddr has been set for the project {}",
                    self.model.name
                ),
            )),
        }
    }

    pub fn project_name(&self) -> &str {
        self.model.name.as_str()
    }

    /// Return the identity of the project's authority
    pub fn authority_identity(&self) -> Option<&Identity> {
        self.authority_identity.as_ref()
    }

    pub fn authority_identifier(&self) -> Option<Identifier> {
        self.authority_identity().map(|i| i.identifier().clone())
    }

    pub fn authority_multiaddr(&self) -> Result<&MultiAddr> {
        match &self.authority_multiaddr {
            Some(authority_multiaddr) => Ok(authority_multiaddr),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!(
                    "no authority route has been configured for the project {}",
                    self.model.name
                ),
            )),
        }
    }

    pub fn space_id(&self) -> &str {
        &self.model.space_id
    }

    pub fn space_name(&self) -> &str {
        &self.model.space_name
    }
}

impl Project {
    pub fn is_admin(&self, user: &UserInfo) -> bool {
        self.model
            .user_roles
            .iter()
            .any(|ur| ur.role == RoleInShare::Admin && ur.email == user.email)
    }

    pub fn is_ready(&self) -> bool {
        self.project_multiaddr.is_some()
            && self.project_identity.is_some()
            && self.authority_multiaddr.is_some()
            && self.authority_identity.is_some()
    }

    pub async fn try_connect_tcp(&self) -> Result<bool> {
        match &self.project_socket_addr {
            None => Ok(false),
            Some(project_socket_addr) => Ok(tokio::net::TcpStream::connect(project_socket_addr)
                .await
                .is_ok()),
        }
    }

    pub fn override_name(&mut self, new_name: String) {
        self.model.name = new_name;
    }
}

impl Output for Project {
    fn item(&self) -> crate::Result<String> {
        let mut f = String::new();
        write!(f, "{}{}", fmt::PADDING, color_primary(self.name()))?;
        writeln!(f, ":")?;
        writeln!(
            f,
            "{}{}Id: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.project_id()
        )?;
        writeln!(
            f,
            "{}{}Name: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.name()
        )?;
        writeln!(
            f,
            "{}{}Space: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.space_name()
        )?;
        writeln!(
            f,
            "{}{}Route: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.project_multiaddr()
                .map(|m| m.to_string())
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Address: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.project_multiaddr()
                .map(|m| TransportRouteResolver::default()
                    .allow_tcp()
                    .socket_address(m)
                    .unwrap_or("N/A".to_string()))
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Identifier: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.project_identifier()
                .map(|i| i.to_string())
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Version: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.model().version.as_deref().unwrap_or("N/A")
        )?;
        writeln!(
            f,
            "{}{}Is running: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.model().running.unwrap_or(false)
        )?;
        writeln!(
            f,
            "{}{}Authority route: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.authority_multiaddr()
                .map(|m| m.to_string())
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Authority address: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.authority_multiaddr()
                .map(|m| TransportRouteResolver::default()
                    .allow_tcp()
                    .socket_address(m)
                    .unwrap_or("N/A".to_string()))
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Authority identifier: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.authority_identifier()
                .map(|i| i.to_string())
                .unwrap_or("N/A".to_string())
        )?;
        writeln!(
            f,
            "{}{}Egress allow list: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.egress_allow_list.join(", ")
        )?;
        Ok(f)
    }

    fn as_list_item(&self) -> crate::Result<String> {
        let mut f = String::new();
        writeln!(f, "Id: {}", self.project_id())?;
        writeln!(f, "Name: {}", self.name())?;
        writeln!(f, "Space: {}", self.space_name())?;
        writeln!(
            f,
            "Route: {}",
            self.project_multiaddr()
                .map(|m| m.to_string())
                .unwrap_or("N/A".to_string())
        )?;
        Ok(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::cloud::enroll::auth0::UserInfo;
    use crate::cloud::project::models::{ProjectModel, ProjectUserRole};
    use crate::cloud::project::Project;
    use crate::cloud::share::{RoleInShare, ShareScope};
    use quickcheck::{Arbitrary, Gen};

    #[tokio::test]
    async fn convert_access_route_to_socket_addr() {
        let mut g = Gen::new(100);
        let mut p = ProjectModel::arbitrary(&mut g);
        p.access_route = "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api".into();
        p.authority_access_route = None;

        let p = Project::import(p).await.unwrap();

        let socket_addr = p.project_socket_addr;
        assert_eq!(socket_addr, Some("node.dnsaddr.com:4000".to_string()));
    }

    #[tokio::test]
    async fn test_is_admin() {
        let mut g = Gen::new(100);
        let mut project = ProjectModel::arbitrary(&mut g);

        project.access_route = "".to_string();
        project.authority_access_route = None;

        // it is possible to test if a user an administrator
        // of the project by comparing the user email and the project role email
        // the email comparison is case insensitive
        project.user_roles = vec![create_admin("test@ockam.io")];

        let project = Project::import(project).await.unwrap();
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
}
