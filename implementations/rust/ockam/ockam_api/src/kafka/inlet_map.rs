use ockam::compat::tokio::sync::Mutex;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Address, AllowAll, Error, IncomingAccessControl, Route};
use ockam_node::tokio::sync::MutexGuard;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

use crate::kafka::kafka_outlet_address;
use crate::kafka::ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS;
use crate::port_range::PortRange;

type BrokerId = i32;

/// Shared structure for every kafka worker to keep track of which brokers are being proxied
/// with the relative inlet listener socket address.
/// Also takes care of creating inlets dynamically when they are not present yet.
#[derive(Clone)]
pub(crate) struct KafkaInletMap {
    inner: Arc<Mutex<KafkaInletMapInner>>,
}

struct KafkaInletMapInner {
    broker_map: HashMap<BrokerId, (Address, SocketAddr)>,
    port_range: PortRange,
    current_port: u16,
    bind_host: String,
    interceptor_route: Route,
    tcp_transport: TcpTransport,
    bootstrap_port: u16,
    bootstrap_worker: Option<Address>,
    access_control: Arc<dyn IncomingAccessControl>,
}

impl KafkaInletMap {
    pub(crate) fn new(
        tcp_transport: TcpTransport,
        access_control: Arc<dyn IncomingAccessControl>,
        interceptor_route: Route,
        bind_address: String,
        bootstrap_port: u16,
        port_range: PortRange,
    ) -> KafkaInletMap {
        Self {
            inner: Arc::new(Mutex::new(KafkaInletMapInner {
                tcp_transport,
                interceptor_route,
                broker_map: HashMap::new(),
                current_port: port_range.start(),
                port_range,
                bootstrap_port,
                bind_host: bind_address,
                bootstrap_worker: None,
                access_control,
            })),
        }
    }

    #[cfg(test)]
    pub(crate) async fn retrieve_inlet(&self, broker_id: BrokerId) -> Option<SocketAddr> {
        let self_guard = self.inner.lock().await;
        self_guard
            .broker_map
            .get(&broker_id)
            .map(|(_worker_address, socket_address)| socket_address.clone())
    }

    /// When the underlying route changes we need to rebuild every existing inlet using the
    /// new route. This method stop existing inlets and create new ones.
    pub(crate) async fn change_route(
        &self,
        context: &Context,
        new_route: Route,
    ) -> ockam::Result<()> {
        let mut new_map = HashMap::new();

        let mut inner = self.inner.lock().await;
        inner.interceptor_route = new_route;

        if let Some(bootstrap_worker_address) = &inner.bootstrap_worker {
            inner
                .tcp_transport
                .stop_inlet(bootstrap_worker_address.clone())
                .await?;
            inner.bootstrap_worker = None;
        }
        self.create_bootstrap_inlet_impl(&mut inner).await?;

        for (broker_id, (worker_address, socket_address)) in &inner.broker_map {
            inner
                .tcp_transport
                .stop_inlet(worker_address.clone())
                .await?;

            let to = route![
                inner.interceptor_route.clone(),
                kafka_outlet_address(*broker_id)
            ];

            let (worker_address, socket_address) = inner
                .tcp_transport
                .create_inlet_impl(socket_address.to_string(), to, inner.access_control.clone())
                .await?;
            new_map.insert(*broker_id, (worker_address, socket_address));
        }

        inner.broker_map = new_map;
        Ok(())
    }

    // pub(crate) async fn create_bootstrap_inlet(&self) -> ockam_core::Result<SocketAddr> {
    //     let mut inner = self.inner.lock().await;
    //     self.create_bootstrap_inlet_impl(&mut inner).await
    // }

    async fn create_bootstrap_inlet_impl<'a>(
        &'a self,
        inner: &mut MutexGuard<'a, KafkaInletMapInner>,
    ) -> ockam_core::Result<SocketAddr> {
        if inner.bootstrap_worker.is_some() {
            return Err(Error::new(
                Origin::Transport,
                Kind::AlreadyExists,
                "bootstrap inlet already exists",
            ));
        }

        let to = route![
            inner.interceptor_route.clone(),
            ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS
        ];

        let (worker_address, socket_address) = inner
            .tcp_transport
            .create_inlet_impl(
                format!("{}:{}", &inner.bind_host, inner.bootstrap_port),
                to,
                inner.access_control.clone(),
            )
            .await?;

        inner.bootstrap_worker = Some(worker_address);
        Ok(socket_address)
    }

    /// Asserts the presence of an inlet for a broker
    /// on first time it'll create the inlet and return the relative address
    /// on the second one it'll just return the address
    pub(crate) async fn assert_inlet_for_broker(
        &self,
        broker_id: BrokerId,
    ) -> ockam_core::Result<SocketAddr> {
        let mut inner = self.inner.lock().await;
        if let Some((_worker_address, socket_address)) = inner.broker_map.get(&broker_id) {
            Ok(*socket_address)
        } else {
            if inner.current_port >= inner.port_range.end() {
                //we don't have any port left for the broker!
                return Err(Error::new(
                    Origin::Transport,
                    Kind::ResourceExhausted,
                    "reached the upper port range",
                ));
            }

            let to = route![
                inner.interceptor_route.clone(),
                kafka_outlet_address(broker_id)
            ];

            let (worker_address, socket_address) = inner
                .tcp_transport
                .create_inlet_impl(
                    format!("{}:{}", inner.bind_host, inner.current_port),
                    to,
                    inner.access_control.clone(),
                )
                .await?;

            inner.current_port += 1;
            inner
                .broker_map
                .insert(broker_id, (worker_address, socket_address));

            Ok(socket_address)
        }
    }
}
