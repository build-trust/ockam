use minicbor::{Decode, Encode};
use tokio::task::JoinHandle;
use tracing as log;

use ockam::{LocalMessage, Route, Worker};
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::{
    route, Address, AllowAll, AsyncTryClone, Decodable, DenyAll, Encodable, Error, Routed, LOCAL,
};
use ockam_node::tokio::sync::mpsc;
use ockam_node::tokio::task::JoinSet;
use ockam_node::tokio::time::{sleep, timeout, Duration};
use ockam_node::Context;
use ockam_node::{tokio, WorkerBuilder};

use crate::nodes::service::default_address::DefaultAddress;
use crate::session::sessions::{ConnectionStatus, Ping, Session};

pub(crate) mod sessions;

const MAX_FAILURES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const DELAY: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Medic {
    retry_delay: Duration,
    delay: Duration,
    sessions: Arc<Mutex<Vec<Session>>>,
    pings: JoinSet<(String, Result<(), Error>)>,
    replacements: JoinSet<(String, Result<Route, Error>)>,
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
pub struct Message {
    #[n(0)] key: String,
    #[n(1)] ping: Ping,
}

impl Medic {
    pub fn new() -> Self {
        Self {
            retry_delay: RETRY_DELAY,
            delay: DELAY,
            sessions: Arc::new(Mutex::new(vec![])),
            pings: JoinSet::new(),
            replacements: JoinSet::new(),
        }
    }

    pub async fn start(
        self,
        ctx: Context,
    ) -> Result<(JoinHandle<()>, Arc<Mutex<Vec<Session>>>), Error> {
        let ctx = ctx
            .new_detached(Address::random_tagged("Medic.ctx"), DenyAll, AllowAll)
            .await?;
        let (tx, rx) = mpsc::channel(32);
        WorkerBuilder::new(Collector(tx))
            .with_address(Collector::address())
            .with_outgoing_access_control(DenyAll)
            .start(&ctx)
            .await?;
        let sessions = self.sessions.clone();
        let handle = tokio::spawn(self.go(ctx, rx));
        Ok((handle, sessions))
    }

    pub async fn stop(ctx: &Context) -> Result<(), Error> {
        ctx.stop_worker(Collector::address()).await
    }

    /// Continuously check all sessions.
    ///
    /// This method never returns. It will ping all healthy sessions and
    /// trigger replacements for the unhealthy ones.
    async fn go(mut self, ctx: Context, mut rx: mpsc::Receiver<Message>) {
        let ctx = Arc::new(ctx);
        loop {
            {
                let mut sessions = self.sessions.lock().unwrap();
                for session in sessions.iter_mut() {
                    let key = session.key().to_string();
                    if session.pings().len() < MAX_FAILURES {
                        let message = Message::new(session.key().to_string());
                        session.add_ping(message.ping);
                        let l = {
                            let v = Encodable::encode(&message).expect("message can be encoded");
                            let echo_route =
                                route![session.ping_route().clone(), DefaultAddress::ECHO_SERVICE];
                            log::trace! {
                                key  = %key,
                                addr = %session.ping_route(),
                                ping = %message.ping,
                                "send ping"
                            }
                            let next = match echo_route.next() {
                                Ok(n) => n,
                                Err(_) => {
                                    log::error! {
                                        key  = %key,
                                        addr = %session.ping_route(),
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
                            LocalMessage::new(
                                echo_route,
                                route![Collector::address()],
                                v,
                                Vec::new(),
                            )
                        };
                        let sender = ctx.clone();
                        self.pings
                            .spawn(async move { (key, sender.send_local_message(l).await) });
                    } else {
                        match session.status() {
                            ConnectionStatus::Up | ConnectionStatus::Down => {
                                log::warn!(%key, "session unresponsive");
                                let f = session.replacement(session.ping_route().clone());
                                session.set_status(ConnectionStatus::Degraded);
                                log::info!(%key, "replacing session");
                                let retry_delay = self.retry_delay;
                                self.replacements.spawn(async move {
                                    sleep(retry_delay).await;
                                    (key, f.await)
                                });
                            }
                            ConnectionStatus::Degraded => {
                                log::warn!(%key, "session is being replaced");
                            }
                        }
                    }
                }
            }

            let _ = timeout(self.delay, self.get_results(&mut rx)).await;
        }
    }

    async fn get_results(&mut self, rx: &mut mpsc::Receiver<Message>) {
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
                    Some(Ok((k, Err(e)))) => {
                        log::warn!(key = %k, err = %e, "replacing session failed");
                        let mut sessions = self.sessions.lock().unwrap();
                        if let Some(s) = sessions.iter_mut().find(|s| s.key() == k) {
                           s.set_status(ConnectionStatus::Down);
                        }
                    }
                    Some(Ok((k, Ok(ping_route)))) => {
                        let mut sessions = self.sessions.lock().unwrap();
                        if let Some(s) = sessions.iter_mut().find(|s| s.key() == k) {
                            log::info!(key = %k, ping_route = %ping_route, "replacement is up");
                            s.set_status(ConnectionStatus::Up);
                            s.set_ping_address(ping_route);
                            s.clear_pings();
                        }
                    }
                },
                Some(m) = rx.recv() => {
                    let mut sessions = self.sessions.lock().unwrap();
                    if let Some(s) = sessions.iter_mut().find(|s| s.key() == m.key) {
                        if s.pings().contains(&m.ping) {
                            log::trace!(key = %m.key, ping = %m.ping, "recv pong");
                            s.clear_pings()
                        }
                    }
                },
                else => {
                    sleep(self.delay).await;
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

pub struct MedicHandle {
    handle: JoinHandle<()>,
    sessions: Arc<Mutex<Vec<Session>>>,
}

impl MedicHandle {
    pub fn new(handle: JoinHandle<()>, sessions: Arc<Mutex<Vec<Session>>>) -> Self {
        Self { handle, sessions }
    }

    pub async fn start_medic(ctx: &Context) -> Result<MedicHandle, Error> {
        let medic = Medic::new();
        let ctx = ctx.async_try_clone().await?;
        let (handle, sessions) = medic.start(ctx).await?;
        let medic_handle = Self::new(handle, sessions);
        Ok(medic_handle)
    }

    pub async fn stop_medic(&self, ctx: &Context) -> Result<(), Error> {
        Medic::stop(ctx).await?;
        self.handle.abort();
        Ok(())
    }

    pub fn add_session(&self, session: Session) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.push(session);
    }

    pub fn remove_session(&self, key: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|s| s.key() != key)
    }

    pub fn status_of(&self, key: &str) -> Option<ConnectionStatus> {
        let sessions = self.sessions.lock().unwrap();
        sessions.iter().find(|s| s.key() == key).map(|s| s.status())
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicBool, Ordering};

    use tracing as log;

    use ockam::{route, Address, Context};
    use ockam_core::compat::sync::Arc;
    use ockam_core::{AsyncTryClone, Result};

    use crate::echoer::Echoer;
    use crate::hop::Hop;
    use crate::session::sessions::ConnectionStatus;
    use crate::session::sessions::Session;
    use crate::session::Medic;

    #[ockam::test]
    async fn test_session_monitoring(ctx: &mut Context) -> Result<()> {
        // Create a new Medic instance
        let medic = Medic::new();

        // Start the Medic in a separate task
        let new_ctx = ctx.async_try_clone().await?;

        let (medic_task, sessions) = medic.start(new_ctx).await?;

        // Medic relies on echo to verify if a session is alive
        ctx.start_worker(Address::from_string("echo"), Echoer)
            .await?;

        // Hop serves as simple neutral address we can use
        ctx.start_worker(Address::from_string("hop"), Hop).await?;

        let replacer_called = Arc::new(AtomicBool::new(false));
        let replacer_can_return = Arc::new(AtomicBool::new(false));

        {
            let mut session = Session::new(route!["broken_route"], "key".to_string());
            let replacer_called = replacer_called.clone();
            let replacer_can_return = replacer_can_return.clone();
            session.set_replacer(Box::new(move |_| {
                let replacer_called = replacer_called.clone();
                let replacer_can_return = replacer_can_return.clone();
                Box::pin(async move {
                    log::info!("replacer called");
                    replacer_called.store(true, Ordering::Release);
                    while !replacer_can_return.load(Ordering::Acquire) {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    // en empty route would do the trick, but a hop is more realistic
                    Ok(route!["hop"])
                })
            }));

            sessions.lock().unwrap().push(session);
        }

        {
            // Initially it's up
            let mut guard = sessions.lock().unwrap();
            let session = guard.iter_mut().next().unwrap();
            assert_eq!(session.status(), ConnectionStatus::Up);
            assert_eq!(session.ping_route(), &route!["broken_route"]);
        }

        // Since the route is broken eventually it will be degraded and will call the replacer
        while !replacer_called.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        {
            // Check the session is now marked as degraded
            let guard = sessions.lock().unwrap();
            let session = guard.iter().next().unwrap();
            assert_eq!(session.status(), ConnectionStatus::Degraded);
            assert_eq!(session.ping_route(), &route!["broken_route"]);
        }

        // Now we allow the replacer to return and replace the route
        replacer_can_return.store(true, Ordering::Release);

        loop {
            {
                // Check that the session is now up, since we don't have any
                // synchronization we keep to keep checking until it's up
                let guard = sessions.lock().unwrap();
                let session = guard.iter().next().unwrap();
                if session.status() == ConnectionStatus::Up {
                    assert_eq!(session.ping_route(), &route!["hop"]);
                    break;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }

        // Shut down the test
        medic_task.abort();
        ctx.stop().await
    }
}
