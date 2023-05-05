use std::str::FromStr;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::cloud::addon::ConfluentConfigResponse;
use ockam::identity::IdentityIdentifier;
use ockam_core::CowStr;
use ockam_core::Result;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;
use ockam_node::tokio;

use crate::error::ApiError;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Default, Clone)]
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

    #[cbor(n(4))]
    pub services: Vec<String>,

    #[cbor(n(5))]
    pub access_route: String,

    #[cbor(n(6))]
    pub users: Vec<String>,

    #[cbor(n(7))]
    pub space_id: String,

    #[cbor(n(8))]
    pub identity: Option<IdentityIdentifier>,

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
}

impl Project {
    pub fn to_owned(&self) -> Project {
        self.clone()
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
        ma.to_socket_addr()
            .map_err(|e| ApiError::generic(&e.to_string()))
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OktaConfig {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(b(0))] pub tag: TypeTag<6434814>,

    #[cbor(b(1))] pub tenant_base_url: String,

    #[cbor(b(2))] pub certificate: String,

    #[cbor(b(3))] pub client_id: String,

    #[cbor(b(4))] pub attributes: Vec<String>,
}

impl<'a> OktaConfig {
    pub fn new<S: ToString, T: AsRef<str>>(
        tenant_base_url: S,
        certificate: S,
        client_id: S,
        attributes: &'a [T],
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant_base_url: tenant_base_url.to_string(),
            certificate: certificate.to_string(),
            client_id: client_id.to_string(),
            attributes: attributes.iter().map(|x| x.as_ref().to_string()).collect(),
        }
    }

    pub fn new_empty_attributes<S: ToString>(
        tenant_base_url: S,
        certificate: S,
        client_id: S,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            tenant_base_url: tenant_base_url.to_string(),
            certificate: certificate.to_string(),
            client_id: client_id.to_string(),
            attributes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OktaAuth0 {
    pub tenant_base_url: String,
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

mod node {
    use std::time::Duration;

    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::{
        BareCloudRequestWrapper, CloudRequestWrapper, ORCHESTRATOR_RESTART_TIMEOUT,
    };
    use crate::nodes::NodeManagerWorker;

    use super::*;

    const TARGET: &str = "ockam_api::cloud::project";

    impl NodeManagerWorker {
        pub(crate) async fn create_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            space_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateProject> = dec.decode()?;
            let cloud_multiaddr = req_wrapper.multiaddr()?;
            let req_body = req_wrapper.req;

            let label = "create_project";
            trace!(target: TARGET, %space_id, project_name = %req_body.name, "creating project");

            let req_builder = Request::post(format!("/v0/{space_id}")).body(&req_body);

            self.request_controller_with_timeout(
                ctx,
                label,
                "create_project",
                &cloud_multiaddr,
                "projects",
                req_builder,
                req_wrapper.identity_name,
                Duration::from_secs(ORCHESTRATOR_RESTART_TIMEOUT),
            )
            .await
        }

        pub(crate) async fn list_projects(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
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

        pub(crate) async fn get_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
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

        pub(crate) async fn delete_project(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
            space_id: &str,
            project_id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
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
}

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use super::*;

    #[derive(Debug, Clone)]
    struct Pr(Project);

    impl Arbitrary for Pr {
        fn arbitrary(g: &mut Gen) -> Self {
            Pr(Project {
                #[cfg(feature = "tag")]
                tag: Default::default(),
                id: String::arbitrary(g),
                name: String::arbitrary(g),
                space_name: String::arbitrary(g),
                services: vec![String::arbitrary(g), String::arbitrary(g)],
                access_route: String::arbitrary(g),
                users: vec![String::arbitrary(g), String::arbitrary(g)],
                space_id: String::arbitrary(g),
                identity: bool::arbitrary(g)
                    .then(|| IdentityIdentifier::from_key_id(&String::arbitrary(g))),
                authority_access_route: bool::arbitrary(g).then(|| String::arbitrary(g)),
                authority_identity: bool::arbitrary(g)
                    .then(|| hex::encode(<Vec<u8>>::arbitrary(g))),
                okta_config: None,
                confluent_config: None,
                version: None,
                running: None,
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

        use crate::schema::SCHEMA;

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
