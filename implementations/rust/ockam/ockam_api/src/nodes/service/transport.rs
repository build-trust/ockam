use crate::nodes::models::transport::{
    CreateTcpConnection, CreateTcpListener, DeleteTransport, TransportList, TransportMode,
    TransportStatus, TransportType,
};
use crate::nodes::service::{random_alias, Alias, ApiTransport, Transports};
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};
use ockam_core::flow_control::FlowControls;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions};

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) fn get_tcp_con_or_list<'a>(
        &self,
        req: &Request<'a>,
        transports: &'a Transports,
        mode: TransportMode,
    ) -> ResponseBuilder<TransportList<'a>> {
        Response::ok(req.id()).body(TransportList::new(
            transports
                .iter()
                .filter(|(_, ApiTransport { tm, .. })| *tm == mode)
                .map(|(tid, api_transport)| {
                    TransportStatus::new(api_transport.clone(), tid.to_string())
                })
                .collect(),
        ))
    }

    pub(super) async fn get_transport<'a>(
        &self,
        req: &Request<'a>,
        id: &str,
        tt: TransportType,
        tm: TransportMode,
    ) -> Result<Vec<u8>> {
        let transport = {
            let inner = self.node_manager.read().await;
            inner.transports.get(id).cloned()
        };
        let res = match transport {
            None => Response::not_found(req.id()).to_vec()?,
            Some(transport) => {
                if transport.tt == tt && transport.tm == tm {
                    Response::ok(req.id())
                        .body(TransportStatus::new(transport, id.to_string()))
                        .to_vec()?
                } else {
                    Response::not_found(req.id()).to_vec()?
                }
            }
        };
        Ok(res)
    }

    pub(super) async fn create_tcp_connection<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        _ctx: &Context,
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateTcpConnection { addr, .. } = dec.decode()?;

        info!("Handling request to create a new TCP connection: {}", addr);
        let socket_addr = addr.to_string();

        let options = TcpConnectionOptions::new();

        let res = node_manager
            .tcp_transport
            .connect(&socket_addr, options)
            .await;

        use {super::TransportType::*, TransportMode::*};

        let response = match res {
            Ok(connection) => {
                let tid = random_alias();
                let api_transport = ApiTransport {
                    tt: Tcp,
                    tm: Connect,
                    socket_address: *connection.socket_address(),
                    worker_address: connection.sender_address().clone(),
                    flow_control_id: connection.flow_control_id().clone(),
                };
                node_manager
                    .transports
                    .insert(tid.clone(), api_transport.clone());
                Response::ok(req.id()).body(TransportStatus::new(api_transport, tid))
            }
            Err(msg) => {
                error!("{}", msg.to_string());
                Response::bad_request(req.id()).body(TransportStatus::new(
                    ApiTransport {
                        tt: Tcp,
                        tm: Connect,
                        socket_address: "0.0.0.0:0000".parse().unwrap(),
                        worker_address: "<none>".into(),
                        flow_control_id: FlowControls::generate_id(), // FIXME
                    },
                    "<none>".to_string(),
                ))
            }
        };

        Ok(response)
    }

    pub(super) async fn create_tcp_listener<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateTcpListener { addr, .. } = dec.decode()?;

        use {super::TransportType::*, TransportMode::*};

        info!("Handling request to create a new tcp listener: {}", addr);

        let options = TcpListenerOptions::new();
        let res = node_manager.tcp_transport.listen(&addr, options).await;

        let response = match res {
            Ok(listener) => {
                let tid = random_alias();
                let api_transport = ApiTransport {
                    tt: Tcp,
                    tm: Listen,
                    socket_address: *listener.socket_address(),
                    worker_address: listener.processor_address().clone(),
                    flow_control_id: listener.flow_control_id().clone(),
                };
                node_manager
                    .transports
                    .insert(tid.clone(), api_transport.clone());
                Response::ok(req.id()).body(TransportStatus::new(api_transport, tid))
            }
            Err(msg) => {
                error!("{}", msg.to_string());
                Response::bad_request(req.id()).body(TransportStatus::new(
                    ApiTransport {
                        tt: Tcp,
                        tm: Listen,
                        socket_address: "0.0.0.0:0000".parse().unwrap(),
                        worker_address: "<none>".into(),
                        flow_control_id: FlowControls::generate_id(), // FIXME
                    },
                    "<none>".to_string(),
                ))
            }
        };

        Ok(response)
    }

    pub(super) async fn delete_transport(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<()>> {
        let mut node_manager = self.node_manager.write().await;
        let body: DeleteTransport = dec.decode()?;
        info!("Handling request to delete transport: {}", body.tid);

        let tid: Alias = body.tid.to_string();

        match node_manager.transports.get(&tid) {
            Some(t) if t.tm == TransportMode::Listen => {
                // FIXME: stopping the listener shuts down the entire node
                // node_manager
                //     .tcp_transport
                //     .stop_listener(&t.worker_address)
                //     .await?;
                node_manager.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            Some(t) => {
                node_manager
                    .tcp_transport
                    .disconnect(&t.worker_address)
                    .await?;
                node_manager.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }
}
