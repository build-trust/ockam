use core::str::FromStr;
use minicbor::Decoder;
use ockam_core::compat::net::IpAddr;

use ockam::compat::tokio::sync::Mutex;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateInlet, CreateOutlet, InletStatus, OutletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use crate::port_range::PortRange;

type BrokerId = i32;

/// Shared structure for every kafka worker to keep track of which brokers are being proxied
/// with the relative outlet socket address.
/// Also takes care of creating outlet dynamically when they are not present yet.
#[derive(Debug, Clone)]
pub(crate) struct KafkaOutletController {
    inner: Arc<Mutex<KafkaOutletMapInner>>,
}

#[derive(Debug)]
struct KafkaOutletMapInner {
    broker_map: HashMap<BrokerId, String>,
}

impl KafkaOutletController {
    pub(crate) fn new() -> KafkaOutletController {
        Self {
            inner: Arc::new(Mutex::new(KafkaOutletMapInner {
                broker_map: HashMap::new(),
            })),
        }
    }

    #[cfg(test)]
    pub(crate) async fn retrieve_outlet(&self, broker_id: BrokerId) -> Option<String> {
        let inner = self.inner.lock().await;
        inner.broker_map.get(&broker_id).copied()
    }

    /// Asserts the presence of an inlet for a broker
    /// on first time it'll create the inlet and return the relative address
    /// on the second one it'll just return the address
    pub(crate) async fn assert_outlet_for_broker(
        &self,
        context: &Context,
        broker_id: BrokerId,
        tcp_address: String,
    ) -> Result<String> {
        let mut inner = self.inner.lock().await;
        if let Some(address) = inner.broker_map.get(&broker_id) {
            Ok(address.clone())
        } else {
            let tcp_address = Self::request_outlet_creation(
                context,
                tcp_address,
                kafka_outlet_address(broker_id),
            )
            .await?;
            inner.broker_map.insert(broker_id, tcp_address.clone());
            Ok(tcp_address)
        }
    }

    async fn request_outlet_creation(
        context: &Context,
        tcp_address: String,
        worker_address: String,
    ) -> Result<String> {
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/outlet")
                    .body(CreateOutlet::new(tcp_address, worker_address, None))
                    .to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: Response = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot create outlet: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid create outlet response",
            ))
        } else {
            let status: OutletStatus = decoder.decode()?;
            Ok(status.tcp_addr.to_string())
        }
    }
}
