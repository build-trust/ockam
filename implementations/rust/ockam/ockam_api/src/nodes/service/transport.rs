use std::collections::BTreeMap;

use crate::nodes::models::transport::{
    CreateTransport, DeleteTransport, TransportList, TransportMode, TransportStatus, TransportType,
};
use crate::nodes::service::{random_alias, Alias};
use minicbor::Decoder;
use ockam::Result;
use ockam_core::api::{Request, Response, ResponseBuilder};

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) fn get_tcp_con_or_list<'a>(
        &self,
        req: &Request<'a>,
        transports: &'a BTreeMap<Alias, (TransportType, TransportMode, String)>,
        mode: TransportMode,
    ) -> ResponseBuilder<TransportList<'a>> {
        Response::ok(req.id()).body(TransportList::new(
            transports
                .iter()
                .filter(|(_, (_, tm, _))| *tm == mode)
                .map(|(tid, (tt, tm, addr))| TransportStatus::new(*tt, *tm, addr, tid))
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
        let addr = addr.to_string();

        let res = match (tt, tm) {
            (Tcp, Listen) => node_manager
                .tcp_transport
                .listen(&addr)
                .await
                .map(|socket| socket.to_string()),
            (Tcp, Connect) => node_manager
                .tcp_transport
                .connect(&addr)
                .await
                .map(|ockam_addr| ockam_addr.address().to_string()),
            _ => unimplemented!(),
        };

        let response = match res {
            Ok(res) => {
                let tid = random_alias();
                node_manager.transports.insert(tid.clone(), (tt, tm, addr));
                Response::ok(req.id()).body(TransportStatus::new(tt, tm, res, tid))
            }
            Err(msg) => Response::bad_request(req.id()).body(TransportStatus::new(
                tt,
                tm,
                msg.to_string(),
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
            // FIXME
            Some(t) if t.1 == TransportMode::Listen => {
                warn!("It is not currently supported to destroy LISTEN transports");
                Ok(Response::bad_request(req.id()))
            }
            Some(_t) => {
                // FIXME node_manager.tcp_transport.disconnect(&t.2).await?;
                node_manager.transports.remove(&tid);
                Ok(Response::ok(req.id()))
            }
            None => Ok(Response::bad_request(req.id())),
        }
    }
}
