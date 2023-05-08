use crate::error::ApiError;
use crate::nodes::models::transport::{
    CreateTcpConnection, CreateTcpListener, DeleteTransport, TransportList, TransportMode,
    TransportStatus, TransportType,
};
use crate::nodes::service::ApiTransport;
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{Address, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Secure, Service, Worker};
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;
use ockam_transport_tcp::{
    TcpConnectionOptions, TcpListenerInfo, TcpListenerOptions, TcpSenderInfo, TcpTransport,
};
use std::net::SocketAddr;

use super::NodeManagerWorker;

fn handle_expose_to_multiaddr(
    tcp: &TcpTransport,
    flow_controls: &FlowControls,
    mut multiaddr: MultiAddr,
) -> Result<(FlowControlId, FlowControlPolicy)> {
    // Tcp connection of listener
    let res = if let Ok(socket) = multiaddr.to_socket_addr() {
        let socket_address = socket.parse::<SocketAddr>().unwrap();
        if let Some(res) = tcp
            .registry()
            .get_all_receiver_processors()
            .iter()
            .find(|x| x.socket_address() == socket_address)
            .map(|x| {
                (
                    x.flow_control_id().clone(),
                    FlowControlPolicy::ProducerAllowMultiple,
                )
            })
        {
            res
        } else if let Some(res) = tcp
            .registry()
            .get_all_listeners()
            .iter()
            .find(|x| x.socket_address() == socket_address)
            .map(|x| {
                (
                    x.flow_control_id().clone(),
                    FlowControlPolicy::SpawnerAllowMultipleMessages,
                )
            })
        {
            res
        } else {
            unimplemented!()
        }
    }
    // Worker or a Secure Channel Listener
    else {
        if multiaddr.len() != 1 {
            unimplemented!()
        }

        let p = multiaddr.pop_front().unwrap(); // FIXME

        match p.code() {
            Worker::CODE => {
                let local = p.cast::<Worker>().unwrap(); // FIXME
                let address = Address::new(LOCAL, &*local);

                let flow_control_id = flow_controls
                    .find_flow_control_with_producer_address(&address)
                    .unwrap() // FIXME
                    .flow_control_id()
                    .clone();

                (flow_control_id, FlowControlPolicy::ProducerAllowMultiple)
            }
            Service::CODE => {
                let local = p.cast::<Service>().unwrap(); // FIXME
                let address = Address::new(LOCAL, &*local);

                let flow_control_id = flow_controls
                    .find_flow_control_with_producer_address(&address)
                    .unwrap() // FIXME
                    .flow_control_id()
                    .clone();

                (flow_control_id, FlowControlPolicy::ProducerAllowMultiple)
            }

            Secure::CODE => {
                let local = p.cast::<Secure>().unwrap(); // FIXME
                let address = Address::new(LOCAL, &*local);

                let flow_control_id = flow_controls
                    .get_flow_control_with_spawner(&address)
                    .unwrap();

                (
                    flow_control_id,
                    FlowControlPolicy::SpawnerAllowMultipleMessages,
                )
            }

            Ip4::CODE | Ip6::CODE | DnsAddr::CODE => {
                unimplemented!()
            }

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol in exposed_to");
                return Err(ApiError::message(
                    "unknown multiaddr protocol in exposed_to",
                ));
            }
        }
    };

    Ok(res)
}

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

    pub(super) fn get_tcp_connections<'a>(
        &self,
        req: &Request<'a>,
        tcp: &TcpTransport,
    ) -> ResponseBuilder<TransportList<'a>> {
        let map = |info: &TcpSenderInfo| {
            TransportStatus::new(ApiTransport {
                tt: TransportType::Tcp,
                tm: (*info.mode()).into(),
                socket_address: info.socket_address(),
                worker_address: info.address().clone(),
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

    pub(super) fn get_tcp_connection<'a>(
        &self,
        req: &Request<'a>,
        tcp: &TcpTransport,
        address: String,
    ) -> ResponseBuilder<TransportStatus<'a>> {
        let sender = match Self::find_connection(tcp, address) {
            None => {
                return Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: TransportType::Tcp,
                    tm: TransportMode::Outgoing,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }));
            }
            Some(sender) => sender,
        };

        let status = TransportStatus::new(ApiTransport {
            tt: TransportType::Tcp,
            tm: (*sender.mode()).into(),
            socket_address: sender.socket_address(),
            worker_address: sender.address().clone(),
            flow_control_id: sender.flow_control_id().clone(),
        });

        Response::ok(req.id()).body(status)
    }

    pub(super) fn get_tcp_listeners<'a>(
        &self,
        req: &Request<'a>,
        tcp: &TcpTransport,
    ) -> ResponseBuilder<TransportList<'a>> {
        let map = |info: &TcpListenerInfo| {
            TransportStatus::new(ApiTransport {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                socket_address: info.socket_address(),
                worker_address: info.address().clone(),
                flow_control_id: info.flow_control_id().clone(),
            })
        };

        Response::ok(req.id()).body(TransportList::new(
            tcp.registry().get_all_listeners().iter().map(map).collect(),
        ))
    }

    pub(super) fn get_tcp_listener<'a>(
        &self,
        req: &Request<'a>,
        tcp: &TcpTransport,
        address: String,
    ) -> ResponseBuilder<TransportStatus<'a>> {
        let listener = match Self::find_listener(tcp, address) {
            None => {
                return Response::bad_request(req.id()).body(TransportStatus::new(ApiTransport {
                    tt: TransportType::Tcp,
                    tm: TransportMode::Listen,
                    socket_address: "0.0.0.0:0000".parse().unwrap(),
                    worker_address: "<none>".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                }));
            }
            Some(listener) => listener,
        };

        let status = TransportStatus::new(ApiTransport {
            tt: TransportType::Tcp,
            tm: TransportMode::Listen,
            socket_address: listener.socket_address(),
            worker_address: listener.address().clone(),
            flow_control_id: listener.flow_control_id().clone(),
        });

        Response::ok(req.id()).body(status)
    }

    pub(super) async fn create_tcp_connection<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
        let node_manager = self.node_manager.read().await;
        let CreateTcpConnection {
            addr, exposed_to, ..
        } = dec.decode()?;

        info!("Handling request to create a new TCP connection: {}", addr);
        let socket_addr = addr.to_string();

        let mut options = TcpConnectionOptions::new();

        for exposed_to in exposed_to {
            let (id, policy) = handle_expose_to_multiaddr(
                &node_manager.tcp_transport,
                ctx.flow_controls(),
                exposed_to,
            )?;
            options = options.as_consumer(&id, policy);
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
                    worker_address: connection.sender_address().clone(),
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
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
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
                    worker_address: listener.processor_address().clone(),
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
