use minicbor::{Decode, Encode};
use tokio::task::JoinHandle;
use tracing as log;

use ockam::{LocalMessage, Route, TransportMessage, Worker};
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::{
    route, Address, AllowAll, AsyncTryClone, Decodable, DenyAll, Encodable, Error, Routed, LOCAL,
};
use ockam_node::tokio::sync::mpsc;
use ockam_node::tokio::task::JoinSet;
use ockam_node::tokio::time::{sleep, timeout, Duration};
use ockam_node::Context;
use ockam_node::{tokio, WorkerBuilder};

use crate::session::sessions::{Key, Ping, Session, Sessions, Status};
use crate::DefaultAddress;

pub(crate) mod sessions;

const MAX_FAILURES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const DELAY: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct Medic {
    retry_delay: Duration,
    delay: Duration,
    sessions: Arc<Mutex<Sessions>>,
    pings: JoinSet<(Key, Result<(), Error>)>,
    replacements: JoinSet<(Key, Result<Route, Error>)>,
}

#[derive(Debug, Copy, Clone, Encode, Decode)]
#[rustfmt::skip]
pub struct Message {
    #[n(0)] key: Key,
    #[n(1)] ping: Ping,
}

impl Medic {
    pub fn new() -> Self {
        Self {
            retry_delay: RETRY_DELAY,
            delay: DELAY,
            sessions: Arc::new(Mutex::new(Sessions::new())),
            pings: JoinSet::new(),
            replacements: JoinSet::new(),
        }
    }

    pub async fn start(
        self,
        ctx: Context,
    ) -> Result<(JoinHandle<()>, Arc<Mutex<Sessions>>), Error> {
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
            log::trace!("check sessions");
            {
                let mut sessions = self.sessions.lock().unwrap();
                for (&key, session) in sessions.iter_mut() {
                    if session.pings().len() < MAX_FAILURES {
                        let m = Message::new(session.key());
                        session.add_ping(m.ping);
                        let l = {
                            let v = Encodable::encode(&m).expect("message can be encoded");
                            let echo_route =
                                route![session.ping_route().clone(), DefaultAddress::ECHO_SERVICE];
                            log::trace! {
                                key  = %key,
                                addr = %session.ping_route(),
                                ping = %m.ping,
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
                            let t = TransportMessage::v1(echo_route, Collector::address(), v);
                            LocalMessage::new(t, Vec::new())
                        };
                        let sender = ctx.clone();
                        self.pings
                            .spawn(async move { (key, sender.forward(l).await) });
                    } else {
                        match session.status() {
                            Status::Up | Status::Down => {
                                log::warn!(%key, "session unresponsive");
                                let f = session.replacement(session.ping_route().clone());
                                session.set_status(Status::Degraded);
                                log::info!(%key, "replacing session");
                                let retry_delay = self.retry_delay;
                                self.replacements.spawn(async move {
                                    sleep(retry_delay).await;
                                    (key, f.await)
                                });
                            }
                            Status::Degraded => {
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
                        if let Some(s) = sessions.session_mut(&k) {
                           s.set_status(Status::Down);
                        }
                    }
                    Some(Ok((k, Ok(ping_route)))) => {
                        let mut sessions = self.sessions.lock().unwrap();
                        if let Some(s) = sessions.session_mut(&k) {
                            log::info!(key = %k, ping_route = %ping_route, "replacement is up");
                            s.set_status(Status::Up);
                            s.set_ping_address(ping_route);
                            s.clear_pings();
                        }
                    }
                },
                Some(m) = rx.recv() => {
                    if let Some(s) = self.sessions.lock().unwrap().session_mut(&m.key) {
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
    fn new(k: Key) -> Self {
        Self {
            key: k,
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
    sessions: Arc<Mutex<Sessions>>,
}

impl MedicHandle {
    pub fn new(handle: JoinHandle<()>, sessions: Arc<Mutex<Sessions>>) -> Self {
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

    pub fn add_session(&self, session: Session) -> Key {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.add(session)
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
    use crate::session::sessions::Session;
    use crate::session::sessions::Status;
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
            let mut session = Session::new(route!["broken_route"]);
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

            sessions.lock().unwrap().add(session);
        }

        {
            // Initially it's up
            let mut guard = sessions.lock().unwrap();
            let (_, session) = guard.iter_mut().next().unwrap();
            assert_eq!(session.status(), Status::Up);
            assert_eq!(session.ping_route(), &route!["broken_route"]);
        }

        // Since the route is broken eventually it will be degraded and will call the replacer
        while !replacer_called.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        {
            // Check the session is now marked as degraded
            let guard = sessions.lock().unwrap();
            let (_, session) = guard.iter().next().unwrap();
            assert_eq!(session.status(), Status::Degraded);
            assert_eq!(session.ping_route(), &route!["broken_route"]);
        }

        // Now we allow the replacer to return and replace the route
        replacer_can_return.store(true, Ordering::Release);

        loop {
            {
                // Check that the session is now up, since we don't have any
                // synchronization we keep to keep checking until it's up
                let guard = sessions.lock().unwrap();
                let (_, session) = guard.iter().next().unwrap();
                if session.status() == Status::Up {
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
