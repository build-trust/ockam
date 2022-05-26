use crate::old::session::error::SessionManagementError;
use crate::old::session::msg::{RequestId, SessionMsg};
use ockam::{Address, Context, DelayedEvent, Result, Route, Routed, Worker};
use std::time::Duration;
use tracing::{error, info, warn};

#[ockam::worker]
pub trait SessionManager: Send + 'static {
    /// Should start session (recommended not exceed timeout)
    async fn start_session(&mut self, ctx: &Context, timeout: Duration) -> Result<Route>;
    /// Should stop session if it exists, should do nothing otherwise
    async fn stop_session(&mut self, ctx: &Context) -> Result<()>;
}

pub struct SessionMaintainer<S: SessionManager> {
    manager: S,
    ping_route: Option<Route>,
    last_sent_request_id: Option<RequestId>,
    heartbeat: DelayedEvent<SessionMsg>,
    heartbeat_duration: Duration,
    session_start_timeout: Duration,
    heartbeat_addr: Address,
    main_addr: Address,
}

impl<S: SessionManager> SessionMaintainer<S> {
    pub async fn start(ctx: &Context, manager: S) -> Result<Address> {
        let heartbeat_addr = Address::random_local();
        let main_addr = Address::random_local();

        let heartbeat =
            DelayedEvent::create(ctx, heartbeat_addr.clone(), SessionMsg::Heartbeat).await?;

        let manager = Self {
            manager,
            ping_route: None,
            last_sent_request_id: None,
            heartbeat,
            heartbeat_duration: Duration::from_secs(60),
            session_start_timeout: Duration::from_secs(30),
            heartbeat_addr: heartbeat_addr.clone(),
            main_addr: main_addr.clone(),
        };

        ctx.start_worker(vec![main_addr.clone(), heartbeat_addr], manager)
            .await?;

        Ok(main_addr)
    }

    #[async_recursion::async_recursion]
    async fn restart_session(&mut self, ctx: &Context) -> Result<()> {
        // Stops session if there is any
        self.heartbeat.cancel();
        self.manager.stop_session(ctx).await?;
        self.last_sent_request_id = None;
        self.ping_route = None;

        // Try to start session
        match self
            .manager
            .start_session(ctx, self.session_start_timeout)
            .await
        {
            Ok(ping_route) => {
                info!(
                    "session started, route to keep-alive responder: {:?}",
                    ping_route,
                );
                // Update ping route
                self.ping_route = Some(ping_route);
                // Schedule heartbeat
                self.heartbeat.schedule(self.heartbeat_duration).await?;
            }
            Err(err) => {
                error!(
                    "Error starting session (will retry in {:?}): {:?}",
                    self.session_start_timeout, err,
                );
                // Sleep for a bit before we try again, to limit the damage if
                // we're stuck unable to start the session indefinitely.
                ctx.sleep(self.session_start_timeout).await;
                self.restart_session(ctx).await?;
            }
        }

        Ok(())
    }
}

#[ockam::worker]
impl<S: SessionManager> Worker for SessionMaintainer<S> {
    type Message = SessionMsg;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        // Start session
        self.restart_session(ctx).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        if msg.msg_addr() == self.main_addr {
            match msg.body() {
                SessionMsg::Pong(request_id) => {
                    let last_request_id = if let Some(id) = self.last_sent_request_id.as_ref() {
                        id
                    } else {
                        // We weren't waiting for any request request_id (may be out-of-order) - ignore
                        warn!("Ignoring keep-alive pong {} (not waiting)", request_id);
                        return Ok(());
                    };

                    if last_request_id != &request_id {
                        // This is not the pong we were waiting for (may be out-of-order) - ignore
                        warn!(
                            "Ignoring keep-alive pong {} (waiting for {})",
                            request_id, last_request_id,
                        );
                        return Ok(());
                    }

                    // Everything is fine
                    info!("Session still alive (got keep-alive pong {})", request_id);
                    self.last_sent_request_id = None;
                    self.heartbeat.schedule(self.heartbeat_duration).await?;
                }
                SessionMsg::Heartbeat | SessionMsg::Ping(_) => {
                    // Shouldn't go to that address
                    return Err(SessionManagementError::MismatchedRequestType.into());
                }
            }
        } else if msg.msg_addr() == self.heartbeat_addr {
            match msg.body() {
                SessionMsg::Heartbeat => {
                    // Heartbeat fired
                    if self.last_sent_request_id.is_some() {
                        // We haven't got pong for latest ping, but heartbeat already fired again
                        info!(
                            "Restarting session: keep-alive ping {} timed_out",
                            self.last_sent_request_id
                                .as_ref()
                                .map(|s| s.0.as_str())
                                .unwrap_or("<unknown>"),
                        );
                        self.restart_session(ctx).await?;
                        return Ok(());
                    }

                    let ping_route = if let Some(r) = self.ping_route.clone() {
                        r
                    } else {
                        info!("Starting or re-starting session: No route to keep-alive responder");
                        // Probably session couldn't start, let's restart it and get new ping_route
                        self.restart_session(ctx).await?;

                        return Ok(());
                    };

                    // Send the next keep-alive ping
                    let request_id = RequestId::generate();
                    info!("Initiating keep-alive, sending ping {}", request_id);
                    ctx.send(ping_route, SessionMsg::Ping(request_id.clone()))
                        .await?;
                    self.last_sent_request_id = Some(request_id);
                    self.heartbeat.schedule(self.heartbeat_duration).await?;
                }
                SessionMsg::Ping(_) | SessionMsg::Pong(_) => {
                    // Shouldn't go to that address
                    return Err(SessionManagementError::MismatchedRequestType.into());
                }
            }
        } else {
            return Err(SessionManagementError::InvalidReceiverAddress.into());
        }

        Ok(())
    }
}
