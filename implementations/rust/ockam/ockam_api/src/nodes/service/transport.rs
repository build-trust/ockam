use crate::nodes::models::transport::{
    CreateTransport, DeleteTransport, TransportList, TransportMode, TransportStatus,
};
use crate::nodes::service::{random_alias, Alias, Transports};
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};

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
                .filter(|(_, (_, tm, _, _))| *tm == mode)
                .map(|(tid, (tt, tm, worker_addr, socket_addr))| {
                    TransportStatus::new(
                        *tt,
                        *tm,
                        socket_addr,
                        worker_addr.address().to_string(),
                        tid.to_string(),
                    )
                })
                .collect(),
        ))
    }

    pub(super) async fn add_transport<'a>(
        &self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder<TransportStatus<'a>>> {
        let mut node_manager = self.node_manager.write().await;
        let CreateTransport { tt, tm, addr, .. } = dec.decode()?;

        use {super::TransportType::*, TransportMode::*};

        info!(
            "Handling request to create a new transport: {}, {}, {}",
            tt, tm, addr
        );
        let socket_addr = addr.to_string();

        let res = match (tt, tm) {
            (Tcp, Listen) => node_manager
                .tcp_transport
                .listen(&addr)
                .await
                .map(|(socket, worker_address)| (socket.to_string(), worker_address)),
            (Tcp, Connect) => node_manager
                .tcp_transport
                .connect(&socket_addr)
                .await
                .map(|worker_address| (socket_addr, worker_address)),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok((socket_address, worker_address)) => {
                let tid = random_alias();
                node_manager.transports.insert(
                    tid.clone(),
                    (tt, tm, worker_address.clone(), socket_address.clone()),
                );
                Response::ok(req.id()).body(TransportStatus::new(
                    tt,
                    tm,
                    socket_address,
                    worker_address.address().to_string(),
                    tid,
                ))
            }
            Err(msg) => Response::bad_request(req.id()).body(TransportStatus::new(
                tt,
                tm,
                msg.to_string(),
                "<none>".to_string(),
                "<none>".to_string(),
            )),
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
            Some(t) if t.1 == TransportMode::Listen => {
                warn!("It is not currently supported to destroy LISTEN transports");
                Ok(Response::bad_request(req.id()))
            }
            Some(t) => {
                node_manager.tcp_transport.disconnect(&t.2).await?;
                node_manager.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }
}
