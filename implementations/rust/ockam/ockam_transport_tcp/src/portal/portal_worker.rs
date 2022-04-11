use crate::{PortalInternalMessage, PortalMessage, TcpPortalRecvProcessor};
use core::time::Duration;
use ockam_core::{async_trait, compat::boxed::Box, Decodable};
use ockam_core::{Address, Any, Result, Route, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, info, trace, warn};

// State change for Outlet: SendPong -> Initialized
// State change for Inlet: SendPing -> ReceivePong -> Initialized

enum State {
    SendPing { ping_route: Route },
    SendPong { pong_route: Route },
    ReceivePong,
    Initialized { onward_route: Route },
}

#[derive(Debug)]
enum TypeName {
    Inlet,
    Outlet,
}

pub(crate) struct TcpPortalWorker {
    state: Option<State>,
    tx: Option<OwnedWriteHalf>,
    rx: Option<OwnedReadHalf>,
    peer: SocketAddr,
    internal_address: Address,
    remote_address: Address,
    receiver_address: Address,
    is_disconnecting: bool,
    type_name: TypeName,
}

impl TcpPortalWorker {
    pub(crate) async fn new_inlet(
        ctx: &Context,
        stream: TcpStream,
        peer: SocketAddr,
        ping_route: Route,
    ) -> Result<()> {
        let _ = Self::start(
            ctx,
            peer,
            State::SendPing { ping_route },
            Some(stream),
            TypeName::Inlet,
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn new_outlet(
        ctx: &Context,
        peer: SocketAddr,
        pong_route: Route,
    ) -> Result<Address> {
        Self::start(
            ctx,
            peer,
            State::SendPong { pong_route },
            None,
            TypeName::Outlet,
        )
        .await
    }

    async fn start(
        ctx: &Context,
        peer: SocketAddr,
        state: State,
        stream: Option<TcpStream>,
        type_name: TypeName,
    ) -> Result<Address> {
        let internal_addr = Address::random_local();
        let remote_addr = Address::random_local();
        let receiver_address = Address::random_local();

        info!(
            "Creating new {:?} at internal: {}, remote: {}",
            type_name, internal_addr, remote_addr
        );

        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        let sender = Self {
            state: Some(state),
            tx,
            rx,
            peer,
            internal_address: internal_addr.clone(),
            remote_address: remote_addr.clone(),
            receiver_address,
            is_disconnecting: false,
            type_name,
        };

        ctx.start_worker(vec![internal_addr, remote_addr.clone()], sender)
            .await?;

        Ok(remote_addr)
    }
}

impl TcpPortalWorker {
    fn take_state(&mut self) -> Result<State> {
        let state = if let Some(s) = self.state.take() {
            s
        } else {
            return Err(TransportError::PortalInvalidState.into());
        };

        Ok(state)
    }

    async fn start_receiver(&mut self, ctx: &Context) -> Result<()> {
        if let Some(rx) = self.rx.take() {
            let receiver = TcpPortalRecvProcessor::new(rx, self.internal_address.clone());
            ctx.start_processor(self.receiver_address.clone(), receiver)
                .await
        } else {
            Err(TransportError::PortalInvalidState.into())
        }
    }

    async fn start_disconnection(
        &mut self,
        ctx: &Context,
        onward_route: Option<Route>,
    ) -> Result<()> {
        self.is_disconnecting = true;

        // Connection was dropped on our side
        if let Some(onward_route) = onward_route {
            // Notify the other end
            ctx.send_from_address(
                onward_route,
                PortalMessage::Disconnect,
                self.remote_address.clone(),
            )
            .await?;

            debug!(
                "Notified the other side from {:?} at: {} about connection drop",
                self.type_name, self.internal_address
            );

            // TODO: Remove when we have better way to handle race condition

            // Avoiding race condition when both inlet and outlet connections
            // are dropped at the same time. In this case we want to wait for the `Disconnect`
            // message from the other side to reach our worker, before we shut it down which
            // leads to errors (destination Worker is already stopped)
            ctx.sleep(Duration::from_secs(1)).await;
        }
        // Connection was dropped on the other side
        else {
            // TODO: Remove when we have better way to handle race condition

            // Avoiding race condition when both inlet and outlet connections
            // are dropped at the same time. In this case Processor may stop itself
            // while we had `Disconnect` message from the other side. Let it stop itself,
            // but recheck that by calling `stop_processor` and ignoring the error
            ctx.sleep(Duration::from_secs(1)).await;

            if ctx
                .stop_processor(self.receiver_address.clone())
                .await
                .is_ok()
            {
                debug!(
                    "{:?} at: {} stopped receiver due to connection drop",
                    self.type_name, self.internal_address
                );
            }
        }

        ctx.stop_worker(self.internal_address.clone()).await?;

        info!(
            "{:?} at: {} stopped due to connection drop",
            self.type_name, self.internal_address
        );

        Ok(())
    }
}

#[async_trait]
impl Worker for TcpPortalWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let state = self.take_state()?;

        match state {
            State::SendPing { ping_route } => {
                // Force creation of Outlet on the other side
                ctx.send_from_address(ping_route, PortalMessage::Ping, self.remote_address.clone())
                    .await?;

                debug!("Inlet at: {} sent ping", self.internal_address);

                self.state = Some(State::ReceivePong);
            }
            State::SendPong { pong_route } => {
                // Respond to Inlet
                ctx.send_from_address(
                    pong_route.clone(),
                    PortalMessage::Pong,
                    self.remote_address.clone(),
                )
                .await?;

                if self.tx.is_none() {
                    let stream = TcpStream::connect(self.peer)
                        .await
                        .map_err(TransportError::from)?;
                    let (rx, tx) = stream.into_split();
                    self.tx = Some(tx);
                    self.rx = Some(rx);

                    self.start_receiver(ctx).await?;

                    debug!(
                        "Outlet at: {} successfully connected",
                        self.internal_address
                    );
                }

                debug!("Outlet at: {} sent pong", self.internal_address);

                self.state = Some(State::Initialized {
                    onward_route: pong_route,
                });
            }
            State::ReceivePong | State::Initialized { .. } => {
                return Err(TransportError::PortalInvalidState.into())
            }
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if self.is_disconnecting {
            return Ok(());
        }

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        let mut onward_route = msg.onward_route();
        let recipient = onward_route.step()?;

        let return_route = msg.return_route();

        if onward_route.next().is_ok() {
            return Err(TransportError::UnknownRoute.into());
        }

        let state = self.take_state()?;

        match state {
            State::ReceivePong => {
                if recipient == self.internal_address {
                    return Err(TransportError::PortalInvalidState.into());
                }

                let msg = PortalMessage::decode(msg.payload())?;

                if let PortalMessage::Pong = msg {
                } else {
                    return Err(TransportError::Protocol.into());
                }

                self.start_receiver(ctx).await?;

                debug!("Inlet at: {} received pong", self.internal_address);

                self.state = Some(State::Initialized {
                    onward_route: return_route,
                });
            }
            State::Initialized { onward_route } => {
                if recipient == self.internal_address {
                    trace!(
                        "{:?} at: {} received internal tcp packet",
                        self.type_name,
                        self.internal_address
                    );

                    let msg = PortalInternalMessage::decode(msg.payload())?;

                    match msg {
                        PortalInternalMessage::Payload(payload) => {
                            ctx.send_from_address(
                                onward_route.clone(),
                                PortalMessage::Payload(payload),
                                self.remote_address.clone(),
                            )
                            .await?;
                        }
                        PortalInternalMessage::Disconnect => {
                            info!(
                                "Tcp stream was dropped for {:?} at: {}",
                                self.type_name, self.internal_address
                            );
                            self.start_disconnection(ctx, Some(onward_route.clone()))
                                .await?;
                        }
                    }
                } else {
                    trace!(
                        "{:?} at: {} received remote tcp packet",
                        self.type_name,
                        self.internal_address
                    );

                    // Send to Tcp stream
                    let msg = PortalMessage::decode(msg.payload())?;

                    match msg {
                        PortalMessage::Payload(payload) => {
                            if let Some(tx) = &mut self.tx {
                                match tx.write_all(&payload).await {
                                    Ok(()) => {}
                                    Err(err) => {
                                        warn!(
                                            "Failed to send message to peer {} with error: {}",
                                            self.peer, err
                                        );
                                        self.start_disconnection(ctx, Some(onward_route.clone()))
                                            .await?;
                                    }
                                }
                            } else {
                                return Err(TransportError::PortalInvalidState.into());
                            }
                        }
                        PortalMessage::Disconnect => {
                            self.start_disconnection(ctx, None).await?;
                        }
                        PortalMessage::Ping | PortalMessage::Pong => {
                            return Err(TransportError::Protocol.into());
                        }
                    }
                }

                self.state = Some(State::Initialized { onward_route })
            }
            State::SendPing { .. } | State::SendPong { .. } => {
                return Err(TransportError::PortalInvalidState.into())
            }
        };

        Ok(())
    }
}
