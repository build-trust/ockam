use std::str::FromStr;

use minicbor::{Decode, Encode};

use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;

use crate::error::ApiError;
use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

pub mod enroll;
pub mod invitation;
pub mod project;
pub mod space;

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
    use minicbor::Encode;

    use ockam_core::{self, Result, Route};
    use ockam_node::Context;

    use crate::nodes::NodeMan;
    use crate::{request, RequestBuilder};

    impl NodeMan {
        pub(crate) async fn request_cloud<T>(
            &self,
            ctx: &mut Context,
            label: &str,
            schema: impl Into<Option<&str>>,
            cloud_route: impl Into<Route>,
            api_service: &str,
            req: RequestBuilder<'_, T>,
        ) -> Result<Vec<u8>>
        where
            T: Encode<()>,
        {
            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), api_service);
            let bytes = {
                let b = request(ctx, label, schema, route, req).await;
                self.delete_secure_channel(ctx, sc).await?;
                b?
            };
            Ok(bytes)
        }
    }
}
