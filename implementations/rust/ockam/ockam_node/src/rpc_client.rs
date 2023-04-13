use crate::{Context, MessageSendReceiveOptions};
use core::time::Duration;
use minicbor::{Decode, Decoder, Encode};
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_core::compat::{fmt, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, DenyAll, Error, Result, Route};

const DEFAULT_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Generic client for request / response communication over a route
pub struct RpcClient {
    ctx: Context,
    route: Route,
    timeout: Duration,
}

impl fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcClient")
            .field("route", &self.route)
            .finish()
    }
}

impl RpcClient {
    /// Create a new RpcClient
    pub async fn new(r: Route, ctx: &Context) -> Result<Self> {
        let ctx = ctx
            .new_detached(Address::random_tagged("RpcClient"), DenyAll, DenyAll)
            .await?;
        Ok(RpcClient {
            ctx,
            route: r,
            timeout: DEFAULT_CLIENT_TIMEOUT,
        })
    }

    /// Specify a timeout for the RpcClient
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }

    /// Make message options
    fn options(&self) -> MessageSendReceiveOptions {
        MessageSendReceiveOptions::new().with_timeout(self.timeout)
    }

    /// Encode request header and body (if any) and send the package to the server.
    pub async fn request<T, R>(&self, req: &RequestBuilder<'_, T>) -> Result<R>
    where
        T: Encode<()>,
        R: for<'a> Decode<'a, ()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;

        let vec = self
            .ctx
            .send_and_receive_extended::<Vec<u8>>(self.route.clone(), buf, self.options())
            .await?
            .body();
        let mut d = Decoder::new(&vec);
        let resp: Response = d.decode()?;
        if resp.status() == Some(Status::Ok) {
            Ok(d.decode()?)
        } else {
            Err(error("request", &resp, &mut d))
        }
    }

    /// Encode request header and body (if any) and send the package to the server.
    pub async fn request_no_resp_body<T>(&self, req: &RequestBuilder<'_, T>) -> Result<()>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        let vec = self
            .ctx
            .send_and_receive_extended::<Vec<u8>>(self.route.clone(), buf, self.options())
            .await?
            .body();
        let mut d = Decoder::new(&vec);
        let resp: Response = d.decode()?;
        if resp.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error("request", &resp, &mut d))
        }
    }
}

/// Decode, log and map response error to ockam_core error.
fn error(label: &str, res: &Response, dec: &mut Decoder<'_>) -> Error {
    if res.has_body() {
        let err = match dec.decode::<ockam_core::api::Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target: "ockam_api::authenticator::direct::client",
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            error  = ?err.message(),
            "<- {label}"
        }
        let msg = err.message().unwrap_or(label);
        Error::new(Origin::Application, Kind::Protocol, msg)
    } else {
        Error::new(Origin::Application, Kind::Protocol, label)
    }
}
