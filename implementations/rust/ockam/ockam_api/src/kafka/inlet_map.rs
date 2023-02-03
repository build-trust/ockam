use core::str::FromStr;
use minicbor::Decoder;

use ockam::compat::tokio::sync::Mutex;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Address, Error, Route};
use ockam_node::Context;

use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use crate::port_range::PortRange;
use crate::route_to_multiaddr;

type BrokerId = i32;

///Shared structure for every kafka worker to keep track of which brokers are being proxied
/// with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Debug, Clone)]
pub(crate) struct KafkaInletMap {
    inner: Arc<Mutex<KafkaInletMapInner>>,
}

#[derive(Debug)]
struct KafkaInletMapInner {
    broker_map: HashMap<BrokerId, SocketAddr>,
    port_range: PortRange,
    current_port: u16,
    bind_host: String,
    interceptor_route: Route,
}

impl KafkaInletMap {
    pub(crate) fn new(
        interceptor_route: Route,
        bind_address: String,
        port_range: PortRange,
    ) -> KafkaInletMap {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                interceptor_route,
                broker_map: HashMap::new(),
                current_port: port_range.start(),
                port_range,
                bind_host: bind_address,
            })),
        }
    }

    #[cfg(test)]
    pub(crate) async fn retrieve_inlet(&self, broker_id: BrokerId) -> Option<SocketAddr> {
        let self_guard = self.inner.lock().await;
        self_guard.broker_map.get(&broker_id).map(|x| x.clone())
    }

    ///assert the presence of an inlet for a broker
    /// on first time it'll create the inlet and return the relative address
    /// on the second one it'll just return the address
    pub(crate) async fn assert_inlet_for_broker(
        &self,
        context: &mut Context,
        broker_id: BrokerId,
    ) -> ockam_core::Result<SocketAddr> {
        let mut self_guard = self.inner.lock().await;
        if let Some(address) = self_guard.broker_map.get(&broker_id) {
            Ok(*address)
        } else {
            if self_guard.current_port >= self_guard.port_range.end() {
                //we don't have any port left for the broker!
                return Err(Error::new(
                    Origin::Transport,
                    Kind::ResourceExhausted,
                    "reached the upper port range",
                ));
            }

            let socket_address = SocketAddr::from_str(&format!(
                "{}:{}",
                self_guard.bind_host, self_guard.current_port
            ))
            .map_err(|err| Error::new(Origin::Transport, Kind::Invalid, err))?;

            let to = route_to_multiaddr(
                &self_guard
                    .interceptor_route
                    .clone()
                    .modify()
                    .append(kafka_outlet_address(broker_id))
                    .into(),
            )
            .ok_or_else(|| {
                Error::new(
                    Origin::Transport,
                    Kind::Invalid,
                    "cannot convert route to multiaddr",
                )
            })?;

            let buffer: Vec<u8> = context
                .send_and_receive(
                    route![NODEMANAGER_ADDR],
                    Request::post("/node/inlet")
                        .body(CreateInlet::to_node(socket_address, to, None, None))
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
                    format!("cannot create inlet: {}", status),
                ));
            }
            let _inlet_address = if !response.has_body() {
                return Err(Error::new(
                    Origin::Transport,
                    Kind::Unknown,
                    "invalid create inlet response",
                ));
            } else {
                let status: InletStatus = decoder.decode()?;
                Address::from(status.worker_addr.to_string())
            };

            self_guard.current_port += 1;
            self_guard.broker_map.insert(broker_id, socket_address);

            Ok(socket_address)
        }
    }
}
