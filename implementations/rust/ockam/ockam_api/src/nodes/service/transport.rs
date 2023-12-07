use std::net::SocketAddr;

use ockam::Result;
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions};

use super::{NodeManager, NodeManagerWorker};
use crate::nodes::models::transport::{
    CreateTcpConnection, CreateTcpListener, DeleteTransport, TransportList, TransportStatus,
};

impl NodeManager {
    fn get_tcp_connections(&self) -> TransportList {
        TransportList::new(
            self.tcp_transport
                .registry()
                .get_all_sender_workers()
                .into_iter()
                .map(TransportStatus::from)
                .collect(),
        )
    }

    fn get_tcp_connection(&self, address: String) -> Option<TransportStatus> {
        let sender = self.tcp_transport().find_connection(address.to_string())?;
        Some(sender.into())
    }

    fn get_tcp_listeners(&self) -> TransportList {
        TransportList::new(
            self.tcp_transport
                .registry()
                .get_all_listeners()
                .into_iter()
                .map(TransportStatus::from)
                .collect(),
        )
    }

    fn get_tcp_listener(&self, address: String) -> Option<TransportStatus> {
        let listener = self.tcp_transport().find_listener(address.to_string())?;
        Some(listener.into())
    }

    async fn create_tcp_connection(
        &self,
        address: String,
        ctx: &Context,
    ) -> Result<TransportStatus> {
        let options = TcpConnectionOptions::new();

        // Add all Hop workers as consumers for Demo purposes
        // Production nodes should not run any Hop workers
        for hop in self.registry.hop_services.keys().await {
            ctx.flow_controls()
                .add_consumer(hop.clone(), &options.flow_control_id());
        }

        let connection = self.tcp_transport.connect(address, options).await?;
        Ok(connection.into())
    }

    async fn create_tcp_listener(&self, address: String) -> Result<TransportStatus> {
        let options = TcpListenerOptions::new();
        let listener = self.tcp_transport.listen(address, options).await?;
        Ok(listener.into())
    }

    async fn delete_tcp_connection(&self, address: String) -> Result<(), String> {
        let sender_address = match address.parse::<SocketAddr>() {
            Ok(socket_address) => self
                .tcp_transport()
                .find_connection_by_socketaddr(socket_address)
                .map(|connection| connection.address().clone())
                .ok_or_else(|| {
                    format!("Connection {socket_address} was not found in the registry.")
                })?,
            Err(_err) => address.into(),
        };

        self.tcp_transport
            .disconnect(sender_address.clone())
            .await
            .map_err(|err| format!("Unable to disconnect from {sender_address}: {err}"))
    }

    async fn delete_tcp_listener(&self, address: String) -> Result<(), String> {
        let listener_address = match address.parse::<SocketAddr>() {
            Ok(socket_address) => self
                .tcp_transport()
                .find_listener_by_socketaddress(socket_address)
                .map(|listener| listener.address().clone())
                .ok_or_else(|| {
                    format!("Listener {socket_address} was not found in the registry.")
                })?,
            Err(_err) => address.into(),
        };

        self.tcp_transport
            .stop_listener(&listener_address)
            .await
            .map_err(|err| format!("Unable to stop listener {listener_address}: {err}"))
    }
}

impl NodeManagerWorker {
    pub(super) async fn get_tcp_connections(&self, req: &RequestHeader) -> Response<TransportList> {
        Response::ok(req).body(self.node_manager.get_tcp_connections())
    }

    pub(super) async fn get_tcp_connection(
        &self,
        req: &RequestHeader,
        address: String,
    ) -> Result<Response<TransportStatus>, Response<Error>> {
        self.node_manager
            .get_tcp_connection(address.to_string())
            .map(|status| Response::ok(req).body(status))
            .ok_or_else(|| {
                let msg = format!("Connection {address} was not found in the registry.");
                Response::not_found(req, &msg)
            })
    }

    pub(super) async fn get_tcp_listeners(&self, req: &RequestHeader) -> Response<TransportList> {
        Response::ok(req).body(self.node_manager.get_tcp_listeners())
    }

    pub(super) async fn get_tcp_listener(
        &self,
        req: &RequestHeader,
        address: String,
    ) -> Result<Response<TransportStatus>, Response<Error>> {
        self.node_manager
            .get_tcp_listener(address.to_string())
            .map(|status| Response::ok(req).body(status))
            .ok_or_else(|| {
                let msg = format!("Listener {address} was not found in the registry.");
                Response::bad_request(req, &msg)
            })
    }

    pub(super) async fn create_tcp_connection<'a>(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        create: CreateTcpConnection,
    ) -> Result<Response<TransportStatus>, Response<Error>> {
        let CreateTcpConnection { addr, .. } = create;
        info!("Handling request to create a new TCP connection: {addr}");

        self.node_manager
            .create_tcp_connection(addr.to_string(), ctx)
            .await
            .map(|status| Response::ok(req).body(status))
            .map_err(|msg| {
                Response::bad_request(req, &format!("Unable to connect to {addr}: {msg}"))
            })
    }

    pub(super) async fn create_tcp_listener<'a>(
        &self,
        req: &RequestHeader,
        create: CreateTcpListener,
    ) -> Result<Response<TransportStatus>, Response<Error>> {
        let CreateTcpListener { addr, .. } = create;
        info!("Handling request to create a new tcp listener: {addr}");

        self.node_manager
            .create_tcp_listener(addr.to_string())
            .await
            .map(|status| Response::ok(req).body(status))
            .map_err(|msg| {
                Response::bad_request(req, &format!("Unable to listen on {addr}: {msg}"))
            })
    }

    pub(super) async fn delete_tcp_connection(
        &self,
        req: &RequestHeader,
        delete: DeleteTransport,
    ) -> Result<Response<()>, Response<Error>> {
        info!("Handling request to stop listener: {}", delete.address);

        self.node_manager
            .delete_tcp_connection(delete.address)
            .await
            .map(|status| Response::ok(req).body(status))
            .map_err(|msg| Response::bad_request(req, &msg))
    }

    pub(super) async fn delete_tcp_listener(
        &self,
        req: &RequestHeader,
        delete: DeleteTransport,
    ) -> Result<Response<()>, Response<Error>> {
        info!("Handling request to stop listener: {}", delete.address);

        self.node_manager
            .delete_tcp_listener(delete.address)
            .await
            .map(|status| Response::ok(req).body(status))
            .map_err(|msg| Response::bad_request(req, &msg))
    }
}
