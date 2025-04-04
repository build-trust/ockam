use crate::nodes::models::policies::{ResourceTypeOrNameOption, SetPolicyRequest};
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::service::messages::SendMessage;
use crate::nodes::service::{encode_response, TARGET};
use crate::nodes::{InMemoryNode, NODEMANAGER_ADDR};
use crate::DefaultAddress;
use ockam_core::api::{Request, Response};
use ockam_core::{Address, Decodable, Result, Routed, Worker};
use ockam_node::Context;
use std::error::Error;
use std::sync::Arc;
use tracing::Level;

#[derive(Clone)]
pub struct NodeManagerWorker {
    pub node_manager: Arc<InMemoryNode>,
}

impl NodeManagerWorker {
    pub fn new(node_manager: Arc<InMemoryNode>) -> Self {
        NodeManagerWorker { node_manager }
    }

    // TODO: This is never called.
    pub async fn stop(&self) -> Result<()> {
        let ctx = self.node_manager.tcp_transport.ctx();
        self.node_manager.stop().await?;
        ctx.stop_address(&NODEMANAGER_ADDR.into())?;
        Ok(())
    }
}

impl NodeManagerWorker {
    //////// Request matching and response handling ////////

    #[instrument(skip_all, fields(method = ?request.header().method(), path = request.header().path()), level = Level::TRACE)]
    async fn handle_request(&mut self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>> {
        let (header, body) = request.into_parts();
        let body = body.unwrap_or_default();
        debug! {
            target: TARGET,
            id     = %header.id(),
            method = ?header.method(),
            path   = %header.path(),
            body   = %header.has_body(),
            "request"
        }

        use ockam_core::api::Method::*;
        let path = header.path();
        let path_segments = header.path_segments::<5>();
        let method = match header.method() {
            Some(m) => m,
            None => todo!(),
        };

        match (method, path_segments.as_slice()) {
            // ==*== Basic node information ==*==
            (Get, ["node"]) => encode_response(&header, self.get_node_status().await),
            (Get, ["node", "resources"]) => {
                encode_response(&header, self.get_node_resources().await)
            }

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => {
                encode_response(&header, Ok(self.get_tcp_connections().await))
            }
            (Get, ["node", "tcp", "connection", address]) => {
                encode_response(&header, self.get_tcp_connection(address.to_string()).await)
            }
            (Post, ["node", "tcp", "connection"]) => encode_response(
                &header,
                self.create_tcp_connection(Decodable::decode(&body)?).await,
            ),
            (Delete, ["node", "tcp", "connection"]) => encode_response(
                &header,
                self.delete_tcp_connection(Decodable::decode(&body)?),
            ),

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => {
                encode_response(&header, Ok(self.get_tcp_listeners().await))
            }
            (Get, ["node", "tcp", "listener", address]) => {
                encode_response(&header, self.get_tcp_listener(address.to_string()).await)
            }
            (Post, ["node", "tcp", "listener"]) => encode_response(
                &header,
                self.create_tcp_listener(Decodable::decode(&body)?).await,
            ),
            (Delete, ["node", "tcp", "listener"]) => {
                encode_response(&header, self.delete_tcp_listener(Decodable::decode(&body)?))
            }

            // ==*== Secure channels ==*==
            (Get, ["node", "secure_channel"]) => {
                encode_response(&header, self.list_secure_channels())
            }
            (Get, ["node", "secure_channel_listener"]) => {
                encode_response(&header, self.list_secure_channel_listener())
            }
            (Post, ["node", "secure_channel"]) => encode_response(
                &header,
                self.create_secure_channel(Decodable::decode(&body)?).await,
            ),
            (Delete, ["node", "secure_channel"]) => encode_response(
                &header,
                self.delete_secure_channel(Decodable::decode(&body)?),
            ),
            (Get, ["node", "show_secure_channel"]) => encode_response(
                &header,
                self.show_secure_channel(Decodable::decode(&body)?).await,
            ),
            (Post, ["node", "secure_channel_listener"]) => encode_response(
                &header,
                self.create_secure_channel_listener(Decodable::decode(&body)?)
                    .await,
            ),
            (Delete, ["node", "secure_channel_listener"]) => encode_response(
                &header,
                self.delete_secure_channel_listener(Decodable::decode(&body)?),
            ),
            (Get, ["node", "show_secure_channel_listener"]) => encode_response(
                &header,
                self.show_secure_channel_listener(Decodable::decode(&body)?),
            ),

            // ==*== Services ==*==
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => encode_response(
                &header,
                self.start_uppercase_service(Decodable::decode(&body)?),
            ),
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => encode_response(
                &header,
                self.start_echoer_service(Decodable::decode(&body)?).await,
            ),
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                encode_response(&header, self.start_hop_service(Decodable::decode(&body)?))
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                &header,
                self.start_kafka_outlet_service(Decodable::decode(&body)?)
                    .await,
            ),
            (Delete, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                &header,
                self.delete_kafka_service(Decodable::decode(&body)?, KafkaServiceKind::Outlet)
                    .await,
            ),
            (Post, ["node", "services", DefaultAddress::KAFKA_INLET]) => encode_response(
                &header,
                self.start_kafka_inlet_service(Decodable::decode(&body)?)
                    .await,
            ),
            (Delete, ["node", "services", DefaultAddress::KAFKA_INLET]) => encode_response(
                &header,
                self.delete_kafka_service(Decodable::decode(&body)?, KafkaServiceKind::Inlet)
                    .await,
            ),
            (Post, ["node", "services", DefaultAddress::HTTP_HEADERS_SERVICE]) => encode_response(
                &header,
                self.start_http_header_service(Decodable::decode(&body)?)
                    .await,
            ),
            (Delete, ["node", "services", DefaultAddress::HTTP_HEADERS_SERVICE]) => {
                encode_response(
                    &header,
                    self.delete_http_overwrite_header_service(Decodable::decode(&body)?)
                        .await,
                )
            }
            (Post, ["node", "services", DefaultAddress::LEASE_MANAGER]) => encode_response(
                &header,
                self.start_influxdb_lease_issuer_service(Decodable::decode(&body)?)
                    .await,
            ),
            (Delete, ["node", "services", DefaultAddress::LEASE_MANAGER]) => encode_response(
                &header,
                self.delete_influxdb_lease_issuer_service(Decodable::decode(&body)?),
            ),
            (Get, ["node", "services"]) => encode_response(&header, self.list_services()),
            (Get, ["node", "services", service_type]) => {
                encode_response(&header, self.list_services_of_type(service_type))
            }

            // ==*== Relay commands ==*==
            (Get, ["node", "relay", alias]) => {
                encode_response(&header, self.show_relay(&header, alias).await)
            }
            (Get, ["node", "relay"]) => encode_response(&header, self.get_relays(&header).await),
            (Delete, ["node", "relay", alias]) => {
                encode_response(&header, self.delete_relay(&header, alias).await)
            }
            (Post, ["node", "relay"]) => encode_response(
                &header,
                self.create_relay(&header, Decodable::decode(&body)?).await,
            ),

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => encode_response(&header, self.get_inlets().await),
            (Get, ["node", "inlet", alias]) => {
                encode_response(&header, self.show_inlet(alias).await)
            }
            (Get, ["node", "outlet"]) => encode_response(&header, self.get_outlets().await),
            (Get, ["node", "outlet", addr]) => {
                let addr: Address = addr.to_string().into();
                encode_response(&header, self.show_outlet(&addr))
            }
            (Post, ["node", "inlet"]) => {
                encode_response(&header, self.create_inlet(Decodable::decode(&body)?).await)
            }
            (Post, ["node", "outlet"]) => {
                encode_response(&header, self.create_outlet(Decodable::decode(&body)?).await)
            }
            (Delete, ["node", "outlet", addr]) => {
                let addr: Address = addr.to_string().into();
                encode_response(&header, self.delete_outlet(&addr).await)
            }
            (Delete, ["node", "inlet", alias]) => {
                encode_response(&header, self.delete_inlet(alias).await)
            }
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== InfluxDB Inlets & Outlets  ==*==
            (Post, ["node", "influxdb_inlet"]) => encode_response(
                &header,
                self.start_influxdb_inlet_service(Decodable::decode(&body)?)
                    .await,
            ),
            (Post, ["node", "influxdb_outlet"]) => encode_response(
                &header,
                self.start_influxdb_outlet_service(Decodable::decode(&body)?)
                    .await,
            ),

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                encode_response(&header, self.add_consumer(Decodable::decode(&body)?).await)
            }

            // ==*== Workers ==*==
            (Get, ["node", "workers"]) => encode_response(&header, self.list_workers().await),

            // ==*== Policies ==*==
            (Post, ["policy", action]) => {
                let payload: SetPolicyRequest = Decodable::decode(&body)?;
                encode_response(
                    &header,
                    self.add_policy(action, payload.resource, payload.expression)
                        .await,
                )
            }
            (Get, ["policy", action]) => encode_response(
                &header,
                self.get_policy(action, Decodable::decode(&body)?).await,
            ),
            (Get, ["policy"]) => {
                let resource_type_or_name_option: ResourceTypeOrNameOption =
                    Decodable::decode(&body)?;
                encode_response(
                    &header,
                    self.list_policies(resource_type_or_name_option.0).await,
                )
            }
            (Delete, ["policy", action]) => encode_response(
                &header,
                self.delete_policy(action, Decodable::decode(&body)?).await,
            ),

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => {
                let send_message: SendMessage<Vec<u8>> = Decodable::decode(&body)?;
                encode_response(
                    &header,
                    self.send_message::<Vec<u8>, Vec<u8>>(send_message).await,
                )
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                encode_response::<Vec<u8>>(
                    &header,
                    Err(Response::bad_request(
                        &header,
                        &format!("Invalid endpoint: {} {}", method, path),
                    )),
                )
            }
        }
    }
}

#[ockam::worker]
impl Worker for NodeManagerWorker {
    type Message = Request<Vec<u8>>;
    type Context = Context;

    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        debug!(target: TARGET, "Shutting down NodeManagerWorker");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Request<Vec<u8>>>,
    ) -> Result<()> {
        let return_route = msg.return_route().clone();
        let request = msg.into_body()?;
        let request_header = request.header().clone();
        let r = match self.handle_request(request).await {
            Ok(r) => r,
            Err(err) => {
                error! {
                    target: TARGET,
                    re     = %request_header.id(),
                    method = ?request_header.method(),
                    path   = %request_header.path(),
                    code   = %err.code(),
                    cause  = ?err.source(),
                    "failed to handle request"
                }
                Response::internal_error(
                    &request_header,
                    &format!("failed to handle request: {err}"),
                )
                .encode_body()?
            }
        };
        debug! {
            target: TARGET,
            re     = %request_header.id(),
            method = ?request_header.method(),
            path   = %request_header.path(),
            "responding"
        }
        ctx.send(return_route, r).await
    }
}
