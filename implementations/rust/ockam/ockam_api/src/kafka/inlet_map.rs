use core::str::FromStr;
use minicbor::Decoder;

use ockam::compat::tokio::sync::Mutex;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Address, AllowAll, Error, Route};
use ockam_node::Context;
use ockam_transport_tcp::InletController;

use crate::kafka::kafka_outlet_address;
use crate::nodes::models::portal::{CreateInlet, InletStatus};
use crate::nodes::NODEMANAGER_ADDR;
use crate::port_range::PortRange;
use crate::route_to_multiaddr;

type BrokerId = i32;

///Shared structure for every kafka worker to keep track of which brokers are being proxied
/// with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Clone)]
pub(crate) struct KafkaInletMap {
    inner: Arc<Mutex<KafkaInletMapInner>>,
}

struct KafkaInletMapInner {
    broker_map: HashMap<BrokerId, SocketAddr>,
    port_range: PortRange,
    current_port: u16,
    bind_host: String,
    interceptor_route: Route,
    inlet_creator: Arc<dyn InletController>,
}

impl KafkaInletMap {
    pub(crate) fn new(
        inlet_creator: Arc<dyn InletController>,
        interceptor_route: Route,
        bind_address: String,
        port_range: PortRange,
    ) -> KafkaInletMap {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                inlet_creator,
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

            let to = self_guard
                .interceptor_route
                .clone()
                .modify()
                .append(kafka_outlet_address(broker_id))
                .into();

            let (_worker_address, socket_address) = self_guard
                .inlet_creator
                .create_inlet(socket_address.to_string(), to, Arc::new(AllowAll))
                .await?;

            self_guard.current_port += 1;
            self_guard.broker_map.insert(broker_id, socket_address);

            Ok(socket_address)
        }
    }
}
