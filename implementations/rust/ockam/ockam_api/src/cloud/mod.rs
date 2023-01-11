use std::str::FromStr;

use minicbor::{Decode, Encode};

use crate::error::ApiError;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowStr, Result, Route};
use ockam_multiaddr::MultiAddr;

pub mod addon;
pub mod enroll;
pub mod project;
pub mod space;
pub mod subscription;

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
///
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

pub type ProjectAddress = CowStr<'static>;

/// A wrapper around a cloud request with extra fields.
#[derive(Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CloudRequestWrapper<'a, T> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8956240>,
    #[b(1)] pub req: T,
    #[b(2)] route: CowStr<'a>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new(req: T, route: &MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            route: route.to_string().into(),
        }
    }

    pub fn route(&self) -> Result<Route> {
        let maddr = MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))?;
        crate::multiaddr_to_route(&maddr)
            .ok_or_else(|| ApiError::generic(&format!("Invalid MultiAddr: {}", maddr)))
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(route: &MultiAddr) -> Self {
        Self::new((), route)
    }
}

mod node {
    use std::env;
    use std::str::FromStr;

    use minicbor::Encode;
    use ockam_vault::Vault;
    use rust_embed::EmbeddedFile;

    use ockam_core::api::RequestBuilder;
    use ockam_core::{self, route, Address, Result, Route};
    use ockam_identity::{Identity, IdentityIdentifier, TrustIdentifierPolicy};
    use ockam_node::api::request;
    use ockam_node::Context;

    use crate::cloud::OCKAM_CONTROLLER_IDENTITY_ID;
    use crate::error::ApiError;
    use crate::nodes::{NodeManager, NodeManagerWorker};
    use crate::StaticFiles;

    const TARGET: &str = "ockam_api::nodemanager::service";

    impl NodeManager {
        /// Load controller identity id from file.
        ///
        /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
        /// load the identity instead of the file.
        pub(crate) fn load_controller_identity_id() -> Result<IdentityIdentifier> {
            if let Ok(s) = env::var(OCKAM_CONTROLLER_IDENTITY_ID) {
                trace!(idt = %s, "Read controller identity id from env");
                return IdentityIdentifier::from_str(&s);
            }
            match StaticFiles::get("controller.id") {
                Some(EmbeddedFile { data, .. }) => {
                    let s = core::str::from_utf8(data.as_ref()).map_err(|err| {
                        ApiError::generic(&format!(
                            "Failed to parse controller identity id: {}",
                            err
                        ))
                    })?;
                    trace!(idt = %s, "Read controller identity id from file");
                    IdentityIdentifier::from_str(s)
                }
                None => Err(ApiError::generic(
                    "Failed to import controller identity id from file",
                )),
            }
        }

        /// Return controller identity's identifier.
        pub(crate) fn controller_identity_id(&self) -> IdentityIdentifier {
            self.controller_identity_id.clone()
        }

        /// Returns a secure channel between the node and the controller.
        async fn controller_secure_channel(
            &mut self,
            route: impl Into<Route>,
            identity: Identity<Vault>,
        ) -> Result<Address> {
            let route = route.into();
            // Create secure channel for the given route using the orchestrator identity.
            trace!(target: TARGET, %route, "Creating orchestrator secure channel");
            let addr = identity
                .create_secure_channel(
                    route,
                    TrustIdentifierPolicy::new(self.controller_identity_id()),
                    &self.authenticated_storage,
                    &self.secure_channel_registry,
                )
                .await?;
            debug!(target: TARGET, %addr, "Orchestrator secure channel created");
            Ok(addr)
        }
    }

    impl NodeManagerWorker {
        #[allow(clippy::too_many_arguments)]
        pub(super) async fn request_controller<T>(
            &mut self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_route: impl Into<Route>,
            api_service: &str,
            req: RequestBuilder<'_, T>,
            ident: Identity<Vault>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let mut node_manger = self.get().write().await;
            let cloud_route = cloud_route.into();
            let sc = node_manger
                .controller_secure_channel(cloud_route, ident)
                .await?;
            let route = route![&sc.to_string(), api_service];
            let res = request(ctx, label, schema, route, req).await;
            ctx.stop_worker(sc).await?;
            res
        }
    }
}
