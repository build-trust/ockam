use std::str::FromStr;

use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{CowStr, Result};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;

pub mod addon;
pub mod enroll;
pub mod lease_manager;
pub mod project;
pub mod space;
pub mod subscription;

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
///
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// A default timeout in seconds
pub const ORCHESTRATOR_RESTART_TIMEOUT: u64 = 180;

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
    #[b(3)] pub identity_name: Option<CowStr<'a>>,
}

impl<'a, T> CloudRequestWrapper<'a, T> {
    pub fn new<S: Into<CowStr<'a>>>(req: T, route: &MultiAddr, identity_name: Option<S>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            req,
            route: route.to_string().into(),
            identity_name: identity_name.map(|x| x.into()),
        }
    }

    pub fn multiaddr(&self) -> Result<MultiAddr> {
        MultiAddr::from_str(self.route.as_ref())
            .map_err(|_err| ApiError::generic(&format!("Invalid route: {}", self.route)))
    }
}

/// A CloudRequestWrapper without an internal request.
pub type BareCloudRequestWrapper<'a> = CloudRequestWrapper<'a, ()>;

impl<'a> BareCloudRequestWrapper<'a> {
    pub fn bare(route: &MultiAddr) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: Default::default(),
            req: (),
            route: route.to_string().into(),
            identity_name: None,
        }
    }
}

mod node {
    use std::str::FromStr;
    use std::time::Duration;

    use minicbor::Encode;
    use rust_embed::EmbeddedFile;

    use ockam::identity::{IdentityIdentifier, SecureChannelOptions, TrustIdentifierPolicy};
    use ockam_core::api::RequestBuilder;
    use ockam_core::env::get_env;
    use ockam_core::{self, route, CowStr, Result};
    use ockam_multiaddr::MultiAddr;
    use ockam_node::api::request_with_options;
    use ockam_node::{Context, MessageSendReceiveOptions, DEFAULT_TIMEOUT};

    use crate::cloud::OCKAM_CONTROLLER_IDENTITY_ID;
    use crate::error::ApiError;
    use crate::nodes::{NodeManager, NodeManagerWorker};
    use crate::StaticFiles;

    impl NodeManager {
        /// Load controller identity id from file.
        ///
        /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
        /// load the identity instead of the file.
        pub(crate) fn load_controller_identity_id() -> Result<IdentityIdentifier> {
            if let Ok(Some(idt)) = get_env::<IdentityIdentifier>(OCKAM_CONTROLLER_IDENTITY_ID) {
                trace!(idt = %idt, "Read controller identity id from env");
                return Ok(idt);
            }
            match StaticFiles::get("controller.id") {
                Some(EmbeddedFile { data, .. }) => {
                    let s = core::str::from_utf8(data.as_ref()).map_err(|err| {
                        ApiError::generic(&format!("Failed to parse controller identity id: {err}"))
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
    }

    impl NodeManagerWorker {
        #[allow(clippy::too_many_arguments)]
        pub(super) async fn request_controller<T>(
            &mut self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_multiaddr: &MultiAddr,
            api_service: &str,
            req: RequestBuilder<'_, T>,
            ident: Option<CowStr<'_>>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            self.request_controller_with_timeout(
                ctx,
                label,
                schema,
                cloud_multiaddr,
                api_service,
                req,
                ident,
                Duration::from_secs(DEFAULT_TIMEOUT),
            )
            .await
        }

        #[allow(clippy::too_many_arguments)]
        pub(super) async fn request_controller_with_timeout<T>(
            &mut self,
            ctx: &Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_multiaddr: &MultiAddr,
            api_service: &str,
            req: RequestBuilder<'_, T>,
            ident: Option<CowStr<'_>>,
            timeout: Duration,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let identity_name = ident.map(|i| i.to_string()).clone();
            let identifier = {
                let node_manager = self.get().read().await;
                node_manager
                    .get_identifier(None, identity_name.clone())
                    .await?
            };

            let secure_channels = {
                let mut node_manager = self.get().write().await;
                node_manager
                    .get_secure_channels(None, identity_name.clone())
                    .await?
            };

            let (sc_address, _sc_flow_control_id) = {
                let node_manager = self.get().read().await;
                let cloud_route =
                    crate::multiaddr_to_route(cloud_multiaddr, &node_manager.tcp_transport)
                        .await
                        .ok_or_else(|| ApiError::generic("Invalid Multiaddr"))?;

                let options = SecureChannelOptions::new().with_trust_policy(
                    TrustIdentifierPolicy::new(node_manager.controller_identity_id()),
                );
                let sc_flow_control_id = options.producer_flow_control_id().clone();
                let sc_address = secure_channels
                    .create_secure_channel(ctx, &identifier, cloud_route.route, options)
                    .await?;

                (sc_address, sc_flow_control_id)
            };

            let route = route![sc_address.clone(), api_service];
            let options = MessageSendReceiveOptions::new().with_timeout(timeout);
            let res = request_with_options(ctx, label, schema, route, req, options).await;
            ctx.stop_worker(sc_address).await?;
            res
        }
    }
}
