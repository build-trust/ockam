use std::str::FromStr;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::cloud::addon::ConfluentConfigResponse;
use ockam_core::AsyncTryClone;
use ockam_core::CowStr;
use ockam_core::Result;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio;

use crate::error::ApiError;
use crate::multiaddr_to_addr;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default)]
#[cbor(map)]
pub struct Project<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<9056532>,

    #[cbor(b(1))]
    #[serde(borrow)]
    pub id: CowStr<'a>,

    #[cbor(b(2))]
    #[serde(borrow)]
    pub name: CowStr<'a>,

    #[cbor(b(3))]
    #[serde(borrow)]
    pub space_name: CowStr<'a>,

    #[cbor(b(4))]
    #[serde(borrow)]
    pub services: Vec<CowStr<'a>>,

    #[cbor(b(5))]
    #[serde(borrow)]
    pub access_route: CowStr<'a>,

    #[cbor(b(6))]
    #[serde(borrow)]
    pub users: Vec<CowStr<'a>>,

    #[cbor(b(7))]
    #[serde(borrow)]
    pub space_id: CowStr<'a>,

    #[cbor(n(8))]
    pub identity: Option<IdentityIdentifier>,

    #[cbor(b(9))]
    #[serde(borrow)]
    pub authority_access_route: Option<CowStr<'a>>,

    #[cbor(b(10))]
    #[serde(borrow)]
    pub authority_identity: Option<CowStr<'a>>,

    #[cbor(b(11))]
    #[serde(borrow)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub okta_config: Option<OktaConfig<'a>>,

    #[cbor(b(12))]
    #[serde(borrow)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confluent_config: Option<ConfluentConfigResponse<'a>>,
}

impl Clone for Project<'_> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl Project<'_> {
    pub fn to_owned<'r>(&self) -> Project<'r> {
        Project {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            id: self.id.to_owned(),
            name: self.name.to_owned(),
            space_name: self.space_name.to_owned(),
            services: self.services.iter().map(|x| x.to_owned()).collect(),
            access_route: self.access_route.to_owned(),
            users: self.users.iter().map(|x| x.to_owned()).collect(),
            space_id: self.space_id.to_owned(),
            identity: self.identity.clone(),
            authority_access_route: self.authority_access_route.as_ref().map(|x| x.to_owned()),
            authority_identity: self.authority_identity.as_ref().map(|x| x.to_owned()),
            okta_config: self.okta_config.as_ref().map(|x| x.to_owned()),
            confluent_config: self.confluent_config.as_ref().map(|x| x.to_owned()),
        }
    }

    pub fn is_ready(&self) -> bool {
        !(self.access_route.is_empty()
            || self.authority_access_route.is_none()
            || self.identity.is_none()
            || self.authority_identity.is_none())
    }

    pub async fn is_reachable(&self) -> Result<bool> {
        let socket_addr = self.access_route_socket_addr()?;
        Ok(tokio::net::TcpStream::connect(&socket_addr).await.is_ok())
    }

    pub fn access_route(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(&self.access_route).map_err(|e| ApiError::generic(&e.to_string()))
    }

    // Converts the `access_route` MultiAddr into a single Address, which will
    // return the host and port of the project node.
    // Ex: if access_route is "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api",
    // then this will return the string "node.dnsaddr.com:4000".
    fn access_route_socket_addr(&self) -> Result<String> {
        let ma = self.access_route()?;
        if let Some(addr) = multiaddr_to_addr(&ma) {
            Ok(addr.address().to_string())
        } else {
            Err(ApiError::generic(
                "Project's access route has not a valid structure",
            ))
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OktaConfig<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<6434814>,

    #[serde(borrow)]
    #[cbor(b(1))] pub tenant_base_url: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(2))] pub certificate: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(3))] pub client_id: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(4))] pub attributes: Vec<CowStr<'a>>,
}

impl<'a> OktaConfig<'a> {
    pub fn new<S: Into<CowStr<'a>>, T: AsRef<str>>(
        tenant_base_url: S,
        certificate: S,
        client_id: S,
        attributes: &'a [T],
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant_base_url: tenant_base_url.into(),
            certificate: certificate.into(),
            client_id: client_id.into(),
            attributes: attributes
                .iter()
                .map(|x| CowStr::from(x.as_ref()))
                .collect(),
        }
    }
}

impl Clone for OktaConfig<'_> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}
impl OktaConfig<'_> {
    pub fn to_owned<'r>(&self) -> OktaConfig<'r> {
        OktaConfig {
            #[cfg(feature = "tag")]
            tag: self.tag.to_owned(),
            tenant_base_url: self.tenant_base_url.to_owned(),
            certificate: self.certificate.to_owned(),
            client_id: self.client_id.to_owned(),
            attributes: self.attributes.iter().map(|x| x.to_owned()).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OktaAuth0 {
    pub tenant_base_url: String,
    pub client_id: String,
    pub certificate: String,
}

impl From<OktaConfig<'_>> for OktaAuth0 {
    fn from(c: OktaConfig) -> Self {
        Self {
            tenant_base_url: c.tenant_base_url.to_string(),
            client_id: c.client_id.to_string(),
            certificate: c.certificate.to_string(),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[rustfmt::skip]
#[cbor(map)]
pub struct InfluxDBTokenLeaseManagerConfig<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))] pub tag: TypeTag<4166488>,

    #[serde(borrow)]
    #[cbor(b(1))] pub endpoint: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(2))] pub token: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(3))] pub org_id: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(4))] pub permissions: CowStr<'a>,

    #[cbor(b(5))] pub max_ttl_secs: i32,

    #[serde(borrow)]
    #[cbor(b(6))] pub user_access_rule: Option<CowStr<'a>>,

    #[serde(borrow)]
    #[cbor(b(7))] pub admin_access_rule: Option<CowStr<'a>>,
}

impl<'a> InfluxDBTokenLeaseManagerConfig<'a> {
    pub fn new<S: Into<CowStr<'a>>>(
        endpoint: S,
        token: S,
        org_id: S,
        permissions: S,
        max_ttl_secs: i32,
        user_access_rule: Option<S>,
        admin_access_rule: Option<S>,
    ) -> Self {
        let uar: Option<CowStr<'a>> = user_access_rule.map(|s| s.into());

        let aar: Option<CowStr<'a>> = admin_access_rule.map(|s| s.into());

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
    pub fn new(
        identity_id: impl Into<CowStr<'a>>,
        description: Option<impl Into<CowStr<'a>>>,
    ) -> Self {
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
    use std::collections::BTreeMap;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cli_state;
    use crate::cli_state::CliState;
    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::config::lookup::ProjectLookup;
    use crate::nodes::{NodeManager, NodeManagerWorker};

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManager {
        pub(crate) fn projects(&self) -> Result<BTreeMap<String, ProjectLookup>> {
            let cli_state = CliState::new()?;
            Ok(cli_state
                .nodes
                .get(&self.node_name)?
                .config
                .lookup
                .projects())
        }
    }

    impl NodeManagerWorker {
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

            let req_builder = Request::post(format!("/v0/{space_id}")).body(&req_body);
            let cli_state = cli_state::CliState::new()?;

            let ident = {
                let inner = self.get().read().await;
                match &req_wrapper.identity_name {
                    Some(existing_identity_name) => {
                        let identity_cfg = cli_state
                            .identities
                            .get(existing_identity_name.as_ref())?
                            .config;
                        match identity_cfg.get(ctx, inner.vault()?).await {
                            Ok(idt) => idt,
                            Err(_) => {
                                let vault_cfg = cli_state.vaults.default()?.config;
                                identity_cfg.get(ctx, &vault_cfg.get().await?).await?
                            }
                        }
                    }
                    None => inner.identity()?.async_try_clone().await?,
                }
            };

            self.request_controller(
                ctx,
                label,
                "create_project",
                cloud_route,
                "projects",
                req_builder,
                ident,
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
            .await
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
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

            let ident = {
                let inner = self.get().read().await;
                inner.identity()?.async_try_clone().await?
            };

            self.request_controller(
                ctx,
                label,
                None,
                cloud_route,
                "projects",
                req_builder,
                ident,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;

    #[derive(Debug, Clone)]
    struct Pr(Project<'static>);

    impl Arbitrary for Pr {
        fn arbitrary(g: &mut Gen) -> Self {
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
                identity: bool::arbitrary(g)
                    .then(|| IdentityIdentifier::from_key_id(&String::arbitrary(g))),
                authority_access_route: bool::arbitrary(g).then(|| String::arbitrary(g).into()),
                authority_identity: bool::arbitrary(g)
                    .then(|| hex::encode(<Vec<u8>>::arbitrary(g)).into()),
                okta_config: None,
                confluent_config: None,
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

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use ockam_core::api::SCHEMA;

        use super::*;

        quickcheck! {
            fn project(o: Pr) -> TestResult {
                let cbor = minicbor::to_vec(o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn projects(o: Vec<Pr>) -> TestResult {
                let empty: Vec<Project> = vec![];
                let cbor = minicbor::to_vec(empty).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Project> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(o).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_project(o: CPr) -> TestResult {
                let cbor = minicbor::to_vec(o.0).unwrap();
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
        let mut p = Pr::arbitrary(&mut g).0;
        p.access_route = "/dnsaddr/node.dnsaddr.com/tcp/4000/service/api".into();

        let socket_addr = p.access_route_socket_addr().unwrap();
        assert_eq!(&socket_addr, "node.dnsaddr.com:4000");
    }
}
