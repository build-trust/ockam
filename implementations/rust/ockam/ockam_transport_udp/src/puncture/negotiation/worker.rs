use crate::puncture::negotiation::message::UdpPunctureNegotiationMessage;
use crate::puncture::rendezvous_service::RendezvousClient;
use crate::{PunctureError, UdpBind, UdpPuncture, UdpPunctureOptions};
use ockam_core::{async_trait, route, Address, Any, Decodable, Route, Routed, Worker};
use ockam_node::Context;
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

pub struct UdpPunctureNegotiationWorker {
    onward_route: Option<Route>,
    is_initiator: bool,
    udp_bind: UdpBind,

    client: RendezvousClient,
    my_udp_public_address: Option<String>,
    their_udp_public_address: Option<String>,

    my_remote_address: Address,
    current_puncture: Option<UdpPuncture>,
    current_handle: Option<JoinHandle<()>>,

    notify_reachable: Option<Sender<Route>>,
}

impl UdpPunctureNegotiationWorker {
    pub fn new_initiator(
        onward_route: Route,
        udp_bind: &UdpBind,
        client: RendezvousClient,
        notify_reachable: Sender<Route>,
    ) -> Self {
        // Will be used as a remote_address of `UdpPunctureReceiverWorker` that we will start
        // after we receive Acknowledge message from the responder
        let my_remote_address =
            Address::random_tagged("UdpPunctureNegotiationWorker.remote.initiator");
        Self {
            onward_route: Some(onward_route),
            is_initiator: true,
            udp_bind: udp_bind.clone(),
            client,
            my_udp_public_address: None,
            their_udp_public_address: None,
            my_remote_address,
            current_puncture: None,
            current_handle: None,
            notify_reachable: Some(notify_reachable),
        }
    }

    pub fn new_responder(udp_bind: &UdpBind, client: RendezvousClient) -> Self {
        // Will be used as a remote_address of `UdpPunctureReceiverWorker` that we will start
        // after we receive Initiate message from the initiator
        let my_remote_address =
            Address::random_tagged("UdpPunctureNegotiationWorker.remote.responder");
        Self {
            onward_route: None,
            is_initiator: false,
            udp_bind: udp_bind.clone(),
            client,
            my_udp_public_address: None,
            their_udp_public_address: None,
            my_remote_address,
            current_puncture: None,
            current_handle: None,
            notify_reachable: None,
        }
    }

    fn role(&self) -> &str {
        if self.is_initiator {
            "initiator"
        } else {
            "responder"
        }
    }
}

#[async_trait]
impl Worker for UdpPunctureNegotiationWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> ockam_core::Result<()> {
        debug!(
            "Initializing UdpPunctureNegotiationWorker {} as {}",
            self.role(),
            ctx.address()
        );
        let my_udp_public_address = self.client.get_my_address().await?;

        info!(
            "UdpPunctureNegotiationWorker {} at {} got its public address: {}",
            self.role(),
            ctx.address(),
            my_udp_public_address
        );

        self.my_udp_public_address = Some(my_udp_public_address.clone());

        if self.is_initiator {
            let onward_route = self
                .onward_route
                .as_ref()
                .ok_or(PunctureError::Internal)?
                .clone();
            // Send Initiate message to the responder, but don't start actual UDP puncture yet,
            // until we receive Acknowledge from them
            ctx.send(
                onward_route,
                UdpPunctureNegotiationMessage::Initiate {
                    initiator_udp_public_address: my_udp_public_address,
                    initiator_remote_address: self.my_remote_address.to_vec(),
                },
            )
            .await?;
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let return_route = msg.return_route();
        let msg = UdpPunctureNegotiationMessage::decode(msg.payload())?;

        info!(
            "UdpPunctureNegotiationWorker {} at {} received msg: {:?}",
            self.role(),
            ctx.address(),
            msg
        );

        match msg {
            UdpPunctureNegotiationMessage::Initiate {
                initiator_udp_public_address,
                initiator_remote_address,
            } => {
                if self.is_initiator {
                    return Err(PunctureError::NegotiationInvalidMessageType)?;
                }

                self.their_udp_public_address = Some(initiator_udp_public_address.clone());
                let initiator_remote_address = Address::from(initiator_remote_address);

                // FIXME: PUNCTURE
                let options = UdpPunctureOptions::new();

                // Let's start puncture as we received the initiates
                UdpPuncture::create(
                    ctx,
                    &self.udp_bind,
                    initiator_udp_public_address,
                    self.my_remote_address.clone(),
                    initiator_remote_address,
                    options,
                    // We can't send messages to the remote address of `UdpPunctureReceiverWorker`
                    // on the other side, since it's not started yet, so we'll just send ping
                    // messages to the corresponding UDP transport worker of that node, the messages
                    // will be just dropped on that side, but the fact that we send them will keep
                    // the "connection" open
                    // After we receive the first ping, which guarantees
                    // that `UdpPunctureReceiverWorker` was started on the other side, we'll start
                    // sending messages to that worker
                    true,
                )
                .await?;

                let my_udp_public_address = self
                    .my_udp_public_address
                    .as_ref()
                    .ok_or(PunctureError::Internal)?
                    .clone();

                // Send Acknowledge back, so that initiator will start the puncture as well
                ctx.send(
                    return_route,
                    UdpPunctureNegotiationMessage::Acknowledge {
                        responder_udp_public_address: my_udp_public_address,
                        responder_remote_address: self.my_remote_address.to_vec(),
                    },
                )
                .await?;
            }
            UdpPunctureNegotiationMessage::Acknowledge {
                responder_udp_public_address,
                responder_remote_address,
            } => {
                if !self.is_initiator {
                    return Err(PunctureError::NegotiationInvalidMessageType)?;
                }

                let responder_remote_address = Address::from(responder_remote_address);
                self.their_udp_public_address = Some(responder_udp_public_address.clone());

                // TODO: PUNCTURE Could have been started at the very beginning inside initialize
                // FIXME: PUNCTURE
                let options = UdpPunctureOptions::new();

                // Start puncture
                let puncture = UdpPuncture::create(
                    ctx,
                    &self.udp_bind,
                    responder_udp_public_address,
                    self.my_remote_address.clone(),
                    responder_remote_address,
                    options,
                    false,
                )
                .await?;

                if let Some(current_handle) = self.current_handle.take() {
                    // Drop the old task
                    current_handle.abort();
                }

                if let Some(notify_reachable) = self.notify_reachable.clone() {
                    // Spawn a task that would notify everyone subscribed with a status of the puncture
                    let mut notify_puncture_open_receiver =
                        puncture.notify_puncture_open_receiver();
                    let route = route![puncture.sender_address()];
                    self.current_handle = Some(tokio::spawn(async move {
                        loop {
                            // FIXME: PUNCTURE is there really a point in that loop?
                            //  the underlying low-level puncture workers won't actually
                            //  re-negotiate the puncture and will just shut down
                            //  So, either make them keep trying instead of shut down, or let's
                            //  shut down here as well and let the higher level code handle
                            //  re-negotiation
                            match notify_puncture_open_receiver.recv().await {
                                Ok(r) => {
                                    if !r.is_empty() {
                                        info!("Notifying UDP puncture is reachable");
                                        _ = notify_reachable.send(route.clone())
                                    } else {
                                        info!("Notifying UDP puncture is NOT reachable");
                                        _ = notify_reachable.send(route![])
                                    }
                                }
                                Err(err) => {
                                    error!("Error receiving UDP puncture notification: {}", err);
                                }
                            }
                        }
                    }));
                }

                self.current_puncture = Some(puncture);
            }
        };

        // TODO: PUNCTURE should we actually shutdown ourselves at this point?

        Ok(())
    }
}
