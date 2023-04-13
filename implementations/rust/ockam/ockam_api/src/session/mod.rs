pub(crate) mod sessions;

use crate::session::sessions::{Key, Ping, Sessions, Status};
use crate::DefaultAddress;
use minicbor::{Decode, Encode};
use ockam::{LocalMessage, Route, TransportMessage, Worker};
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{route, Address, AllowAll, Decodable, DenyAll, Encodable, Error, Routed, LOCAL};
use ockam_node::tokio;
use ockam_node::tokio::sync::mpsc;
use ockam_node::tokio::task::JoinSet;
use ockam_node::tokio::time::{sleep, timeout, Duration};
use ockam_node::Context;
use tracing as log;

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

    pub fn sessions(&self) -> Arc<Mutex<Sessions>> {
        self.sessions.clone()
    }

    pub async fn start(self, ctx: Context) -> Result<(), Error> {
        let ctx = ctx
            .new_detached(Address::random_tagged("Medic.ctx"), DenyAll, AllowAll)
            .await?;
        let (tx, rx) = mpsc::channel(32);
        ctx.start_worker(Collector::address(), Collector(tx), AllowAll, DenyAll)
            .await?;
        self.go(ctx, rx).await
    }

    /// Continuously check all sessions.
    ///
    /// This method never returns. It will ping all healthy sessions and
    /// trigger replacements for the unhealthy ones.
    async fn go(mut self, ctx: Context, mut rx: mpsc::Receiver<Message>) -> ! {
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
                                ctx.flow_controls().add_consumer(
                                    Collector::address(),
                                    &flow_control_id,
                                    FlowControlPolicy::ProducerAllowMultiple,
                                );
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
                                self.replacements.spawn(async move {
                                    sleep(self.retry_delay).await;
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
                else => break
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
