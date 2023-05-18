use crate::nodes::models::transport::{
    CreateTcpConnection, CreateTcpListener, DeleteTransport, TransportList, TransportMode,
    TransportStatus, TransportType,
};
use crate::nodes::service::ApiTransport;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_core::Address;
use ockam_node::Context;
use ockam_transport_tcp::{
    TcpConnectionOptions, TcpListenerInfo, TcpListenerOptions, TcpSenderInfo, TcpTransport,
};
use std::net::SocketAddr;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    fn find_connection(tcp: &TcpTransport, address: String) -> Option<TcpSenderInfo> {
        match address.parse::<SocketAddr>() {
            Ok(socket_address) => tcp
                .registry()
                .get_all_sender_workers()
                .iter()
                .find(|x| x.socket_address() == socket_address)
                .cloned(),
            Err(_err) => {
                let address: Address = address.into();

                // Check if it's a Receiver Address
                let address = if let Some(receiver) = tcp
                    .registry()
                    .get_all_receiver_processors()
                    .iter()
                    .find(|x| x.address() == &address)
                {
                    receiver.sender_address().clone()
                } else {
                    address
                };

                tcp.registry()
                    .get_all_sender_workers()
                    .iter()
                    .find(|x| x.address() == &address)
                    .cloned()
            }
        }
    }

    fn find_listener(tcp: &TcpTransport, address: String) -> Option<TcpListenerInfo> {
        match address.parse::<SocketAddr>() {
            Ok(socket_address) => tcp
                .registry()
                .get_all_listeners()
                .iter()
                .find(|x| x.socket_address() == socket_address)
                .cloned(),
            Err(_err) => {
                let address: Address = address.into();

                tcp.registry()
                    .get_all_listeners()
                    .iter()
                    .find(|x| x.address() == &address)
                    .cloned()
            }
        }
    }

    pub(super) fn get_tcp_connections(
        &self,
        req: &Request,
        tcp: &TcpTransport,
    ) -> ResponseBuilder<TransportList> {
        let map = |info: &TcpSenderInfo| {
            TransportStatus::new(ApiTransport {
                tt: TransportType::Tcp,
                tm: (*info.mode()).into(),
                socket_address: info.socket_address(),
                worker_address: info.address().to_string(),
                processor_address: info.receiver_address().to_string(),
                flow_control_id: info.flow_control_id().clone(),
            })
        };

        Response::ok(req.id()).body(TransportList::new(
            tcp.registry()
                .get_all_sender_workers()
                .iter()
                .map(map)
                .collect(),
        ))
    }

    pub(super) fn get_tcp_connection(
        &self,
        req: &Request,
        tcp: &TcpTransport,
        address: String,
    ) -> ResponseBuilder<TransportStatus> {
        let sender = match Self::find_connection(tcp, address) {
            None => {
                return Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: TransportType::Tcp,
                    tm: TransportMode::Outgoing,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    processor_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }));
            }
            Some(sender) => sender,
        };

        let status = TransportStatus::new(ApiTransport {
            tt: TransportType::Tcp,
            tm: (*sender.mode()).into(),
            socket_address: sender.socket_address(),
            worker_address: sender.address().to_string(),
            processor_address: sender.receiver_address().to_string(),
            flow_control_id: sender.flow_control_id().clone(),
        });

        Response::ok(req.id()).body(status)
    }

    pub(super) fn get_tcp_listeners(
        &self,
        req: &Request,
        tcp: &TcpTransport,
    ) -> ResponseBuilder<TransportList> {
        let map = |info: &TcpListenerInfo| {
            TransportStatus::new(ApiTransport {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                socket_address: info.socket_address(),
                worker_address: "<none>".into(),
                processor_address: info.address().to_string(),
                flow_control_id: info.flow_control_id().clone(),
            })
        };

        Response::ok(req.id()).body(TransportList::new(
            tcp.registry().get_all_listeners().iter().map(map).collect(),
        ))
    }

    pub(super) fn get_tcp_listener(
        &self,
        req: &Request,
        tcp: &TcpTransport,
        address: String,
    ) -> ResponseBuilder<TransportStatus> {
        let listener = match Self::find_listener(tcp, address) {
            None => {
                return Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: TransportType::Tcp,
                    tm: TransportMode::Listen,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    processor_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }));
            }
            Some(listener) => listener,
        };

        let status = TransportStatus::new(ApiTransport {
            tt: TransportType::Tcp,
            tm: TransportMode::Listen,
            socket_address: listener.socket_address(),
            worker_address: "<none>".into(),
            processor_address: listener.address().to_string(),
            flow_control_id: listener.flow_control_id().clone(),
        });

        Response::ok(req.id()).body(status)
    }

    pub(super) async fn create_tcp_connection<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<TransportStatus>> {
        let node_manager = self.node_manager.read().await;
        let CreateTcpConnection { addr, .. } = dec.decode()?;

        info!("Handling request to create a new TCP connection: {}", addr);
        let socket_addr = addr.to_string();

        let options = TcpConnectionOptions::new();

        // Add all Hop workers as consumers for Demo purposes
        // Production nodes should not run any Hop workers
        for hop in node_manager.registry.hop_services.keys() {
            ctx.flow_controls().add_consumer(
                hop.clone(),
                &options.producer_flow_control_id(),
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        let res = node_manager
            .tcp_transport
            .connect(&socket_addr, options)
            .await;

        use {super::TransportType::*, TransportMode::*};

        let response = match res {
            Ok(connection) => {
                let api_transport = ApiTransport {
                    tt: Tcp,
                    tm: Outgoing,
                    socket_address: *connection.socket_address(),
                    worker_address: connection.sender_address().to_string(),
                    processor_address: connection.receiver_address().to_string(),
                    flow_control_id: connection.flow_control_id().clone(),
                };
                Response::ok(req.id()).body(TransportStatus::new(api_transport))
            }
            Err(msg) => {
                error!("{}", msg.to_string());
                Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: Tcp,
                    tm: Outgoing,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    processor_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }))
            }
        };

        Ok(response)
    }

    pub(super) async fn create_tcp_listener<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus>> {
        let node_manager = self.node_manager.read().await;
        let CreateTcpListener { addr, .. } = dec.decode()?;

        use {super::TransportType::*, TransportMode::*};

        info!("Handling request to create a new tcp listener: {}", addr);

        let options = TcpListenerOptions::new();
        let res = node_manager.tcp_transport.listen(&addr, options).await;

        let response = match res {
            Ok(listener) => {
                let api_transport = ApiTransport {
                    tt: Tcp,
                    tm: Listen,
                    socket_address: *listener.socket_address(),
                    worker_address: "<none>".into(),
                    processor_address: listener.processor_address().to_string(),
                    flow_control_id: listener.flow_control_id().clone(),
                };
                Response::ok(req.id()).body(TransportStatus::new(api_transport))
            }
            Err(msg) => {
                error!("{}", msg.to_string());
                Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: Tcp,
                    tm: Listen,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    processor_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }))
            }
        };

        Ok(response)
    }

    pub(super) async fn delete_tcp_connection(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let node_manager = self.node_manager.read().await;
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to stop listener: {}", body.address);

        let sender_address = match body.address.parse::<SocketAddr>() {
            Ok(socket_address) => {
                match node_manager
                    .tcp_transport
                    .registry()
                    .get_all_sender_workers()
                    .iter()
                    .find(|x| x.socket_address() == socket_address)
                    .map(|x| x.address().clone())
                {
                    None => return Ok(Response::bad_request(req.id())),
                    Some(addr) => addr,
                }
            }
            Err(_err) => body.address.into(),
        };

        match node_manager.tcp_transport.disconnect(&sender_address).await {
            Ok(_) => Ok(Response::ok(req.id())),
            Err(_err) => Ok(Response::bad_request(req.id())),
        }
    }

    pub(super) async fn delete_tcp_listener(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let node_manager = self.node_manager.read().await;
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to stop listener: {}", body.address);

        let listener_address = match body.address.parse::<SocketAddr>() {
            Ok(socket_address) => {
                match node_manager
                    .tcp_transport
                    .registry()
                    .get_all_listeners()
                    .iter()
                    .find(|x| x.socket_address() == socket_address)
                    .map(|x| x.address().clone())
                {
                    None => return Ok(Response::bad_request(req.id())),
                    Some(addr) => addr,
                }
            }
            Err(_err) => body.address.into(),
        };

        match node_manager
            .tcp_transport
            .stop_listener(&listener_address)
            .await
        {
            Ok(_) => Ok(Response::ok(req.id())),
            Err(_err) => Ok(Response::bad_request(req.id())),
        }
    }
}
