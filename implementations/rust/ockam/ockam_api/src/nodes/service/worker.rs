use crate::nodes::models::policies::SetPolicyRequest;
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::service::{encode_response, TARGET};
use crate::nodes::{InMemoryNode, NODEMANAGER_ADDR};
use crate::DefaultAddress;
use minicbor::Decoder;
use ockam_core::api::{RequestHeader, Response};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct NodeManagerWorker {
    pub node_manager: Arc<InMemoryNode>,
}

impl NodeManagerWorker {
    pub fn new(node_manager: Arc<InMemoryNode>) -> Self {
        NodeManagerWorker { node_manager }
    }

    // TODO: This is never called.
    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        self.node_manager.stop(ctx).await?;
        ctx.stop_address(&NODEMANAGER_ADDR.into())?;
        Ok(())
    }
}

impl NodeManagerWorker {
    //////// Request matching and response handling ////////

    #[instrument(skip_all, fields(method = ?req.method(), path = req.path()))]
    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        debug! {
            target: TARGET,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use ockam_core::api::Method::*;
        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        let r = match (method, path_segments.as_slice()) {
            // ==*== Basic node information ==*==
            (Get, ["node"]) => encode_response(req, self.get_node_status().await)?,
            (Get, ["node", "resources"]) => encode_response(req, self.get_node_resources().await)?,

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => self.get_tcp_connections(req).await.to_vec()?,
            (Get, ["node", "tcp", "connection", address]) => {
                encode_response(req, self.get_tcp_connection(address.to_string()).await)?
            }
            (Post, ["node", "tcp", "connection"]) => {
                encode_response(req, self.create_tcp_connection(ctx, dec.decode()?).await)?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                encode_response(req, self.delete_tcp_connection(dec.decode()?))?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => self.get_tcp_listeners(req).await.to_vec()?,
            (Get, ["node", "tcp", "listener", address]) => {
                encode_response(req, self.get_tcp_listener(address.to_string()).await)?
            }
            (Post, ["node", "tcp", "listener"]) => {
                encode_response(req, self.create_tcp_listener(dec.decode()?).await)?
            }
            (Delete, ["node", "tcp", "listener"]) => {
                encode_response(req, self.delete_tcp_listener(dec.decode()?))?
            }

            // ==*== Secure channels ==*==
            (Get, ["node", "secure_channel"]) => encode_response(req, self.list_secure_channels())?,
            (Get, ["node", "secure_channel_listener"]) => {
                encode_response(req, self.list_secure_channel_listener())?
            }
            (Post, ["node", "secure_channel"]) => {
                encode_response(req, self.create_secure_channel(dec.decode()?, ctx).await)?
            }
            (Delete, ["node", "secure_channel"]) => {
                encode_response(req, self.delete_secure_channel(dec.decode()?, ctx))?
            }
            (Get, ["node", "show_secure_channel"]) => {
                encode_response(req, self.show_secure_channel(dec.decode()?))?
            }
            (Post, ["node", "secure_channel_listener"]) => encode_response(
                req,
                self.create_secure_channel_listener(dec.decode()?, ctx)
                    .await,
            )?,
            (Delete, ["node", "secure_channel_listener"]) => {
                encode_response(req, self.delete_secure_channel_listener(dec.decode()?, ctx))?
            }
            (Get, ["node", "show_secure_channel_listener"]) => {
                encode_response(req, self.show_secure_channel_listener(dec.decode()?))?
            }

            // ==*== Services ==*==
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => {
                encode_response(req, self.start_uppercase_service(ctx, dec.decode()?))?
            }
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => {
                encode_response(req, self.start_echoer_service(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                encode_response(req, self.start_hop_service(ctx, dec.decode()?))?
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                req,
                self.start_kafka_outlet_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Outlet)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_INLET]) => encode_response(
                req,
                self.start_kafka_inlet_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_INLET]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Inlet)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::LEASE_MANAGER]) => encode_response(
                req,
                self.start_influxdb_lease_issuer_service(ctx, dec.decode()?)
                    .await,
            )?,
            (Delete, ["node", "services", DefaultAddress::LEASE_MANAGER]) => encode_response(
                req,
                self.delete_influxdb_lease_issuer_service(ctx, dec.decode()?),
            )?,
            (Get, ["node", "services"]) => encode_response(req, self.list_services())?,
            (Get, ["node", "services", service_type]) => {
                encode_response(req, self.list_services_of_type(service_type))?
            }

            // ==*== Relay commands ==*==
            (Get, ["node", "relay", alias]) => {
                encode_response(req, self.show_relay(req, alias).await)?
            }
            (Get, ["node", "relay"]) => encode_response(req, self.get_relays(req).await)?,
            (Delete, ["node", "relay", alias]) => {
                encode_response(req, self.delete_relay(req, alias).await)?
            }
            (Post, ["node", "relay"]) => {
                encode_response(req, self.create_relay(ctx, req, dec.decode()?).await)?
            }

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => encode_response(req, self.get_inlets().await)?,
            (Get, ["node", "inlet", alias]) => encode_response(req, self.show_inlet(alias).await)?,
            (Get, ["node", "outlet"]) => self.get_outlets(req).to_vec()?,
            (Get, ["node", "outlet", addr]) => {
                let addr: Address = addr.to_string().into();
                encode_response(req, self.show_outlet(&addr))?
            }
            (Post, ["node", "inlet"]) => {
                encode_response(req, self.create_inlet(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "outlet"]) => {
                encode_response(req, self.create_outlet(ctx, dec.decode()?).await)?
            }
            (Delete, ["node", "outlet", addr]) => {
                let addr: Address = addr.to_string().into();
                encode_response(req, self.delete_outlet(&addr).await)?
            }
            (Delete, ["node", "inlet", alias]) => {
                encode_response(req, self.delete_inlet(alias).await)?
            }
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== InfluxDB Inlets & Outlets  ==*==
            (Post, ["node", "influxdb_inlet"]) => encode_response(
                req,
                self.start_influxdb_inlet_service(ctx, dec.decode()?).await,
            )?,
            (Post, ["node", "influxdb_outlet"]) => encode_response(
                req,
                self.start_influxdb_outlet_service(ctx, dec.decode()?).await,
            )?,

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                encode_response(req, self.add_consumer(ctx, dec.decode()?).await)?
            }

            // ==*== Workers ==*==
            (Get, ["node", "workers"]) => encode_response(req, self.list_workers(ctx).await)?,

            // ==*== Policies ==*==
            (Post, ["policy", action]) => {
                let payload: SetPolicyRequest = dec.decode()?;
                encode_response(
                    req,
                    self.add_policy(action, payload.resource, payload.expression)
                        .await,
                )?
            }
            (Get, ["policy", action]) => {
                encode_response(req, self.get_policy(action, dec.decode()?).await)?
            }
            (Get, ["policy"]) => encode_response(req, self.list_policies(dec.decode()?).await)?,
            (Delete, ["policy", action]) => {
                encode_response(req, self.delete_policy(action, dec.decode()?).await)?
            }

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => {
                encode_response(req, self.send_message(ctx, dec.decode()?).await)?
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req, &format!("Invalid endpoint: {} {}", method, path))
                    .to_vec()?
            }
        };
        Ok(r)
    }
}

#[ockam::worker]
impl Worker for NodeManagerWorker {
    type Message = Vec<u8>;
    type Context = Context;

    async fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        debug!(target: TARGET, "Shutting down NodeManagerWorker");
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let return_route = msg.return_route().clone();
        let body = msg.into_body()?;
        let mut dec = Decoder::new(&body);
        let req: RequestHeader = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let r = match self.handle_request(ctx, &req, &mut dec).await {
            Ok(r) => r,
            Err(err) => {
                error! {
                    target: TARGET,
                    re     = %req.id(),
                    method = ?req.method(),
                    path   = %req.path(),
                    code   = %err.code(),
                    cause  = ?err.source(),
                    "failed to handle request"
                }
                Response::internal_error(&req, &format!("failed to handle request: {err} {req:?}"))
                    .to_vec()?
            }
        };
        debug! {
            target: TARGET,
            re     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            "responding"
        }
        ctx.send(return_route, r).await
    }
}
