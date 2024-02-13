use minicbor::{Decode, Encode};
use tokio::task::JoinHandle;
use tracing as log;

use crate::nodes::registry::Registry;
use ockam::{LocalMessage, Worker};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    route, Address, AllowAll, AsyncTryClone, Decodable, DenyAll, Encodable, Error, Routed, LOCAL,
};
use ockam_node::tokio::sync::mpsc;
use ockam_node::tokio::task::JoinSet;
use ockam_node::tokio::time::{sleep, timeout, Duration};
use ockam_node::Context;
use ockam_node::{tokio, WorkerBuilder};

use crate::nodes::service::default_address::DefaultAddress;
use crate::session::sessions::{ConnectionStatus, Ping, ReplacerOutcome, Session};

pub(crate) mod sessions;

const MAX_FAILURES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const PING_INTERVAL: Duration = Duration::from_secs(3);

pub struct Medic {
    retry_delay: Duration,
    ping_interval: Duration,
    registry: Arc<Registry>,
    pings: JoinSet<(String, Result<(), Error>)>,
    replacements: JoinSet<(String, Result<ReplacerOutcome, Error>)>,
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub struct Message {
    #[n(0)] key: String,
    #[n(1)] ping: Ping,
}

impl Medic {
    pub fn new(registry: Arc<Registry>) -> Self {
        Self {
            retry_delay: RETRY_DELAY,
            ping_interval: PING_INTERVAL,
            registry,
            pings: JoinSet::new(),
            replacements: JoinSet::new(),
        }
    }

    pub async fn start(self, ctx: Context) -> Result<JoinHandle<()>, Error> {
        let ctx = ctx
            .new_detached(Address::random_tagged("Medic.ctx"), DenyAll, AllowAll)
            .await?;
        let (tx, ping_receiver) = mpsc::channel(32);
        WorkerBuilder::new(Collector(tx))
            .with_address(Collector::address())
            .with_outgoing_access_control(DenyAll)
            .start(&ctx)
            .await?;
        let handle = tokio::spawn(self.check_loop(ctx, ping_receiver));
        Ok(handle)
    }

    pub async fn stop(ctx: &Context) -> Result<(), Error> {
        ctx.stop_worker(Collector::address()).await
    }

    /// Continuously check all sessions.
    ///
    /// This method never returns. It will ping all healthy sessions and
    /// trigger replacements for the unhealthy ones.
    async fn check_loop(mut self, ctx: Context, mut ping_receiver: mpsc::Receiver<Message>) {
        let ctx = Arc::new(ctx);
        loop {
            log::trace!("check sessions");
            // explicitly scoping the lock to release it before the sleep
            {
                let sessions = self.sessions().await;

                for session in sessions {
                    let key = session.key().to_string();
                    if session.pings().len() < MAX_FAILURES {
                        let message = Message::new(session.key().to_string());
                        session.add_ping(message.ping);
                        let encoded_message =
                            Encodable::encode(&message).expect("message can be encoded");

                        // if the session is up, send a ping
                        if let Some(ping_route) = session.ping_route().clone() {
                            let echo_route =
                                route![ping_route.clone(), DefaultAddress::ECHO_SERVICE];
                            log::trace! {
                                key  = %key,
                                addr = %ping_route,
                                ping = %message.ping,
                                "send ping"
                            }

                            let next = match echo_route.next() {
                                Ok(n) => n,
                                Err(_) => {
                                    log::error! {
                                        key  = %key,
                                        addr = %ping_route,
                                        "empty route"
                                    }
                                    continue;
                                }
                            };
                            if let Some(flow_control_id) = ctx
                                .flow_controls()
                                .find_flow_control_with_producer_address(next)
                                .map(|x| x.flow_control_id().clone())
                            {
                                ctx.flow_controls()
                                    .add_consumer(Collector::address(), &flow_control_id);
                            }
                            let local_message = LocalMessage::new()
                                .with_onward_route(echo_route)
                                .with_return_route(route![Collector::address()])
                                .with_payload(encoded_message);

                            let sender = ctx.clone();
                            self.pings.spawn(async move {
                                log::trace!("sending ping");
                                (key, sender.forward(local_message).await)
                            });
                        };
                    } else {
                        // We reached the maximum number of failures
                        match session.connection_status() {
                            ConnectionStatus::Up | ConnectionStatus::Down => {
                                log::warn!(%key, "session unresponsive");
                                session.degraded();
                                let replacer = session.replacer();
                                log::info!(%key, "replacing session");
                                let retry_delay = self.retry_delay;
                                self.replacements.spawn(async move {
                                    sleep(retry_delay).await;
                                    (key, replacer.recreate().await)
                                });
                            }
                            ConnectionStatus::Degraded => {
                                log::warn!(%key, "session is being replaced");
                            }
                        }
                    }
                }
            }

            let _ = timeout(self.ping_interval, self.get_results(&mut ping_receiver)).await;
        }
    }

    async fn sessions(&self) -> Vec<Session> {
        let inlets_values = self.registry.inlets.values().await;
        let inlets = inlets_values.iter().map(|info| info.session.clone());

        let relay_values = self.registry.relays.values().await;
        let relays = relay_values.iter().map(|info| info.session.clone());

        inlets.chain(relays).collect()
    }

    async fn session(&self, key: &str) -> Option<Session> {
        let inlets_values = self.registry.inlets.values().await;
        let inlets = inlets_values.iter().find(|info| info.session.key() == key);
        if let Some(info) = inlets {
            return Some(info.session.clone());
        }

        let relay_values = self.registry.relays.values().await;
        let relays = relay_values.iter().find(|info| info.session.key() == key);
        if let Some(info) = relays {
            return Some(info.session.clone());
        }

        None
    }

    async fn get_results(&mut self, ping_receiver: &mut mpsc::Receiver<Message>) {
        loop {
            tokio::select! {
                p = self.pings.join_next(), if !self.pings.is_empty() => match p {
                    None                  => log::debug!("no pings to send"),
                    Some(Err(e))          => log::error!("task failed: {e:?}"),
                    Some(Ok((k, Err(e)))) => log::debug!(key = %k, err = %e, "failed to send ping"),
                    Some(Ok((k, Ok(())))) => log::trace!(key = %k, "sent ping"),
                },
                r = self.replacements.join_next(), if !self.replacements.is_empty() => match r {
                    None                  => log::debug!("no replacements"),
                    Some(Err(e))          => log::error!("task failed: {e:?}"),
                    Some(Ok((key, Err(err)))) => {
                        log::warn!(key = %key, err = %err, "replacing session failed");
                        if let Some(session) = self.session(&key).await {
                           session.down();
                        }
                    }
                    Some(Ok((key, Ok(replacer_outcome)))) => {
                        if let Some(session) = self.session(&key).await {
                            log::info!(key = %key, ping_route = %replacer_outcome.ping_route, "replacement is up");
                            session.clear_pings();
                            session.up(replacer_outcome);
                        }
                    }
                },
                Some(message) = ping_receiver.recv() => {
                    log::trace!("received pong");
                    if let Some(session) = self.session(&message.key).await {
                        if session.pings().contains(&message.ping) {
                            log::trace!(key = %message.key, ping = %message.ping, "recv pong");
                            session.clear_pings()
                        }
                    }
                },
                else => {
                    sleep(self.ping_interval).await;
                    break
                }
            }
        }
    }
}

impl Message {
    fn new(key: String) -> Self {
        Self {
            key,
            ping: Ping::new(),
        }
    }
}

impl Encodable for Message {
    fn encode(&self) -> Result<Vec<u8>, Error> {
        minicbor::to_vec(self).map_err(Error::from)
    }
}

impl Decodable for Message {
    fn decode(m: &[u8]) -> Result<Self, Error> {
        minicbor::decode(m).map_err(Error::from)
    }
}

impl ockam_core::Message for Message {}

/// A collector receives echo messages and forwards them.
#[derive(Debug)]
struct Collector(mpsc::Sender<Message>);

impl Collector {
    const NAME: &'static str = "ockam.ping.collector";

    fn address() -> Address {
        Address::new(LOCAL, Self::NAME)
    }
}

#[ockam::worker]
impl Worker for Collector {
    type Message = Message;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<(), Error> {
        if self.0.send(msg.body()).await.is_err() {
            log::debug!("collector could not send message to medic")
        }
        Ok(())
    }
}

pub(crate) struct MedicHandle {
    handle: JoinHandle<()>,
}

impl MedicHandle {
    pub fn new(handle: JoinHandle<()>) -> Self {
        Self { handle }
    }

    pub async fn start_medic(ctx: &Context, registry: Arc<Registry>) -> Result<MedicHandle, Error> {
        let medic = Medic::new(registry);
        let ctx = ctx.async_try_clone().await?;
        let handle = medic.start(ctx).await?;
        let medic_handle = Self::new(handle);
        Ok(medic_handle)
    }

    pub async fn stop_medic(&self, ctx: &Context) -> Result<(), Error> {
        Medic::stop(ctx).await?;
        self.handle.abort();
        Ok(())
    }

    pub async fn connect(session: &mut Session) -> Result<ReplacerOutcome, Error> {
        let outcome = session.replacer().recreate().await?;
        session.up(outcome.clone());
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicBool, Ordering};

    use ockam::{route, Address, Context};
    use ockam_core::compat::sync::Arc;
    use ockam_core::{async_trait, AsyncTryClone, Error, Result};
    use ockam_multiaddr::MultiAddr;

    use crate::echoer::Echoer;
    use crate::hop::Hop;
    use crate::nodes::registry::Registry;
    use crate::session::sessions::{ConnectionStatus, ReplacerOutcome, SessionReplacer};
    use crate::session::sessions::{CurrentInletStatus, ReplacerOutputKind, Session};
    use crate::session::Medic;

    #[derive(Clone)]
    struct MockReplacer {
        pub called: Arc<AtomicBool>,
        pub can_return: Arc<AtomicBool>,
    }

    impl MockReplacer {
        pub fn new() -> Self {
            Self {
                called: Arc::new(AtomicBool::new(false)),
                can_return: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait]
    impl SessionReplacer for MockReplacer {
        async fn create(&mut self) -> std::result::Result<ReplacerOutcome, Error> {
            self.called.store(true, Ordering::Release);
            while !self.can_return.load(Ordering::Acquire) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Ok(ReplacerOutcome {
                ping_route: route!["hop"],
                kind: ReplacerOutputKind::Inlet(CurrentInletStatus {
                    route: route!["hop"],
                    worker: Address::from_string("echo"),
                    connection_status: ConnectionStatus::Up,
                }),
            })
        }

        async fn close(&mut self) {}
    }

    #[ockam::test]
    async fn test_session_monitoring(ctx: &mut Context) -> Result<()> {
        let registry = Arc::new(Registry::default());

        // Create a new Medic instance
        let medic = Medic::new(registry.clone());

        // Start the Medic in a separate task
        let new_ctx = ctx.async_try_clone().await?;

        let medic_task = medic.start(new_ctx).await?;

        // Medic relies on echo to verify if a session is alive
        ctx.start_worker(Address::from_string("echo"), Echoer)
            .await?;

        // Hop serves as simple neutral address we can use
        ctx.start_worker(Address::from_string("hop"), Hop).await?;

        let mock_replacer = MockReplacer::new();
        let session = Session::new(mock_replacer.clone());

        // by default session is down
        assert_eq!(session.connection_status(), ConnectionStatus::Down);
        assert_eq!(session.ping_route(), None);

        // mark the session as up
        session.up(ReplacerOutcome {
            ping_route: route!["broken_route"],
            kind: ReplacerOutputKind::Inlet(CurrentInletStatus {
                route: route!["broken_route"],
                worker: Address::from_string("mock-address"),
                connection_status: ConnectionStatus::Up,
            }),
        });

        assert_eq!(session.connection_status(), ConnectionStatus::Up);
        assert_eq!(session.ping_route().unwrap(), route!["broken_route"]);

        registry
            .inlets
            .insert(
                "inlet-1".to_string(),
                crate::nodes::registry::InletInfo {
                    bind_addr: "127.0.0.1:10000".to_string(),
                    outlet_addr: MultiAddr::default(),
                    session: session.clone(),
                },
            )
            .await;

        // Since the route is broken eventually it will be degraded and will call the replacer
        while !mock_replacer.called.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Check the session is now marked as degraded
        assert_eq!(session.connection_status(), ConnectionStatus::Degraded);
        assert_eq!(session.ping_route(), None);

        // Now we allow the replacer to return and replace the route
        mock_replacer.can_return.store(true, Ordering::Release);

        loop {
            // Check that the session is now up, since we don't have any
            // synchronization we keep to keep checking until it's up
            if session.connection_status() == ConnectionStatus::Up {
                assert_eq!(session.ping_route().unwrap(), route!["hop"]);
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        // Shut down the test
        medic_task.abort();
        ctx.stop().await
    }
}
