use rand::random;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as SyncMutex;

use crate::nodes::service::default_address::DefaultAddress;
use crate::session::collector::Collector;
use crate::session::connection_status::ConnectionStatus;
use crate::session::ping::Ping;
use crate::session::replacer::{AdditionalSessionReplacer, ReplacerOutputKind, SessionReplacer};
use crate::session::status::{Status, StatusInternal};

use ockam::LocalMessage;
use ockam_core::compat::sync::Arc;
use ockam_core::{route, Address, AllowAll, DenyAll, Encodable};
use ockam_core::{Result, Route};
use ockam_node::compat::asynchronous::Mutex as AsyncMutex;
use ockam_node::tokio::sync::mpsc;
use ockam_node::tokio::task::JoinHandle;
use ockam_node::tokio::time::{sleep, Duration};
use ockam_node::Context;
use ockam_node::{tokio, WorkerBuilder};

const MAX_FAILURES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const PING_INTERVAL: Duration = Duration::from_secs(10);

/// State that is accessed from multiple places/threads, therefore needs to be wrapper in Arc<Mutex<>>
#[derive(Clone)]
struct SharedState {
    /// Current status
    status: Status, // internal locking is present
    /// Indicates if replacement is running
    is_being_replaced: Arc<AtomicBool>,
    /// Outcome of last session creation
    last_outcome: Arc<SyncMutex<Option<ReplacerOutputKind>>>,
    /// Replacer impl
    replacer: Arc<AsyncMutex<dyn SessionReplacer>>,
    /// Pings that we sent. The whole list is cleared upon receiving an ack
    sent_pings: Arc<AsyncMutex<Vec<Ping>>>,
}

/// State that is accessed from multiple places/threads, therefore needs to be wrapper in Arc<Mutex<>>
#[derive(Clone)]
struct AdditionalSharedState {
    /// Current status
    status: Status, // internal locking is present
    /// Indicates if replacement is running
    is_being_replaced: Arc<AtomicBool>,
    /// Replacer impl
    replacer: Arc<AsyncMutex<dyn AdditionalSessionReplacer>>,
    /// Pings that we sent. The whole list is cleared upon receiving an ack
    sent_pings: Arc<AsyncMutex<Vec<Ping>>>,
}

/// State to support additional routes (like UDP puncture for an Inlet)
struct AdditionalState {
    enable_fallback: bool,
    retry_delay: Duration,
    ping_interval: Duration,
    collector_address: Address,
    ping_receiver_handle: Option<JoinHandle<()>>,
    run_loop_handle: Option<JoinHandle<()>>,
    shared_state: AdditionalSharedState,
}

pub struct AdditionalSessionOptions {
    replacer: Arc<AsyncMutex<dyn AdditionalSessionReplacer>>,
    enable_fallback: bool,
    retry_delay: Duration,
    ping_interval: Duration,
}

impl AdditionalSessionOptions {
    pub fn new(
        replacer: Arc<AsyncMutex<dyn AdditionalSessionReplacer>>,
        enable_fallback: bool,
        retry_delay: Duration,
        ping_interval: Duration,
    ) -> Self {
        Self {
            replacer,
            enable_fallback,
            retry_delay,
            ping_interval,
        }
    }

    pub fn create(
        replacer: Arc<AsyncMutex<dyn AdditionalSessionReplacer>>,
        enable_fallback: bool,
    ) -> Self {
        Self {
            replacer,
            enable_fallback,
            retry_delay: RETRY_DELAY,
            ping_interval: PING_INTERVAL,
        }
    }
}

/// Monitors individual session
pub struct Session {
    ctx: Context,
    key: String, // Solely for debug purposes/logging
    /// Delay before we attempt to recreate the session if the previous attempt failed
    retry_delay: Duration,
    ping_interval: Duration,
    initial_connect_was_called: bool,

    collector_address: Address,

    shared_state: SharedState,

    run_loop_handle: Option<JoinHandle<()>>,
    ping_receiver_handle: Option<JoinHandle<()>>,

    additional_state: Option<AdditionalState>,
}

impl Session {
    /// Make initial connection [`Session::start_monitoring`] should be called after
    pub async fn initial_connect(&mut self) -> Result<ReplacerOutputKind> {
        let outcome = self.shared_state.replacer.lock().await.create().await?;
        self.shared_state.status.set_up(outcome.ping_route);
        self.shared_state.last_outcome = Arc::new(SyncMutex::new(Some(outcome.kind.clone())));

        if let Some(additional_state) = self.additional_state.as_mut() {
            if !additional_state.enable_fallback {
                // We need to establish additional connection now
                let additional_ping_route = additional_state
                    .shared_state
                    .replacer
                    .lock()
                    .await
                    .create_additional()
                    .await?;

                additional_state
                    .shared_state
                    .status
                    .set_up(additional_ping_route);
            }
        }

        self.initial_connect_was_called = true;

        Ok(outcome.kind)
    }

    /// Create a Session
    pub async fn create(
        ctx: &Context,
        replacer: Arc<AsyncMutex<dyn SessionReplacer>>,
        additional_session_options: Option<AdditionalSessionOptions>,
    ) -> Result<Self> {
        Self::create_extended(
            ctx,
            replacer,
            additional_session_options,
            RETRY_DELAY,
            PING_INTERVAL,
        )
        .await
    }

    /// Create a Session
    pub async fn create_extended(
        ctx: &Context,
        replacer: Arc<AsyncMutex<dyn SessionReplacer>>,
        additional_session_options: Option<AdditionalSessionOptions>,
        retry_delay: Duration,
        ping_interval: Duration,
    ) -> Result<Self> {
        let ctx = ctx
            .new_detached(Address::random_tagged("Session.ctx"), DenyAll, AllowAll)
            .await?;

        Ok(Self::new(
            ctx,
            replacer,
            additional_session_options,
            retry_delay,
            ping_interval,
        ))
    }

    pub fn new(
        ctx: Context,
        replacer: Arc<AsyncMutex<dyn SessionReplacer>>,
        additional_session_options: Option<AdditionalSessionOptions>,
        retry_delay: Duration,
        ping_interval: Duration,
    ) -> Self {
        let key = hex::encode(random::<[u8; 8]>());
        let collector_address = Address::random_tagged(&format!("Collector.{}", key));

        let shared_state = SharedState {
            status: Default::default(),
            is_being_replaced: Arc::new(AtomicBool::new(false)),
            last_outcome: Arc::new(SyncMutex::new(None)),
            replacer: replacer.clone(),
            sent_pings: Default::default(),
        };

        let additional_state = if let Some(additional_session_options) = additional_session_options
        {
            let shared_state = AdditionalSharedState {
                status: Default::default(),
                is_being_replaced: Arc::new(AtomicBool::new(false)),
                replacer: additional_session_options.replacer,
                sent_pings: Default::default(),
            };

            let additional_collector_address =
                Address::random_tagged(&format!("Collector.{}.additional", key));

            Some(AdditionalState {
                enable_fallback: additional_session_options.enable_fallback,
                retry_delay: additional_session_options.retry_delay,
                ping_interval: additional_session_options.ping_interval,
                collector_address: additional_collector_address,
                ping_receiver_handle: None,
                run_loop_handle: None,
                shared_state,
            })
        } else {
            None
        };

        Self {
            ctx,
            key,
            collector_address,
            retry_delay,
            ping_interval,

            initial_connect_was_called: false,

            shared_state,

            run_loop_handle: None,
            ping_receiver_handle: None,

            additional_state,
        }
    }

    /// Current connection status
    pub fn connection_status(&self) -> ConnectionStatus {
        self.shared_state.status.connection_status()
    }

    /// Indicates that session is being replaced at the momment
    pub fn is_being_replaced(&self) -> bool {
        self.shared_state.is_being_replaced.load(Ordering::Relaxed)
    }

    /// Current connection status
    pub fn additional_connection_status(&self) -> Option<ConnectionStatus> {
        self.additional_state
            .as_ref()
            .map(|additional_state| additional_state.shared_state.status.connection_status())
    }

    /// Indicates that session is being replaced at the momment
    pub fn additional_is_being_replaced(&self) -> Option<bool> {
        self.additional_state.as_ref().map(|additional_state| {
            additional_state
                .shared_state
                .is_being_replaced
                .load(Ordering::Relaxed)
        })
    }

    /// Last session creation outcome
    pub fn last_outcome(&self) -> Option<ReplacerOutputKind> {
        self.shared_state.last_outcome.lock().unwrap().clone()
    }

    /// Start monitoring the session
    pub async fn start_monitoring(&mut self) -> Result<()> {
        let (ping_channel_sender, ping_channel_receiver) = mpsc::channel(1);

        // Will shut down itself when we stop the Collector
        self.ping_receiver_handle = Some(tokio::spawn(Self::wait_for_pings(
            self.key.clone(),
            ping_channel_receiver,
            self.shared_state.sent_pings.clone(),
        )));

        WorkerBuilder::new(Collector::new(ping_channel_sender))
            .with_address(self.collector_address.clone())
            .with_outgoing_access_control(DenyAll)
            .start(&self.ctx)
            .await?;

        let ctx = self
            .ctx
            .new_detached(
                Address::random_tagged("Session.ctx.run_loop"),
                DenyAll,
                AllowAll,
            )
            .await?;

        let handle = tokio::spawn(Self::run_loop(
            ctx,
            self.key.clone(),
            self.initial_connect_was_called,
            self.collector_address.clone(),
            self.shared_state.clone(),
            self.ping_interval,
            self.retry_delay,
        ));

        self.run_loop_handle = Some(handle);

        if let Some(additional_state) = self.additional_state.as_mut() {
            let (ping_channel_sender, ping_channel_receiver) = mpsc::channel(1);

            // Will shut down itself when we stop the Collector
            additional_state.ping_receiver_handle = Some(tokio::spawn(Self::wait_for_pings(
                self.key.clone(),
                ping_channel_receiver,
                additional_state.shared_state.sent_pings.clone(),
            )));

            WorkerBuilder::new(Collector::new(ping_channel_sender))
                .with_address(additional_state.collector_address.clone())
                .with_outgoing_access_control(DenyAll)
                .start(&self.ctx)
                .await?;

            let ctx = self
                .ctx
                .new_detached(
                    Address::random_tagged("Session.ctx.run_loop.additional"),
                    DenyAll,
                    AllowAll,
                )
                .await?;

            let handle = tokio::spawn(Self::run_loop_additional(
                ctx,
                self.key.clone(),
                self.initial_connect_was_called && !additional_state.enable_fallback,
                self.shared_state.clone(),
                additional_state.enable_fallback,
                additional_state.shared_state.clone(),
                additional_state.collector_address.clone(),
                additional_state.ping_interval,
                additional_state.retry_delay,
            ));

            additional_state.run_loop_handle = Some(handle);
        }

        Ok(())
    }

    async fn stop_additional(&mut self) {
        if let Some(mut additional_state) = self.additional_state.take() {
            if let Some(run_loop_handle) = additional_state.run_loop_handle.take() {
                run_loop_handle.abort();
            }

            // We're shutting down everything, so let's not fallback to the main connection
            let enable_fallback = false;
            additional_state
                .shared_state
                .replacer
                .lock()
                .await
                .close_additional(enable_fallback)
                .await;
            additional_state.shared_state.status.set_down();

            _ = self
                .ctx
                .stop_worker(additional_state.collector_address)
                .await;
        }
    }

    async fn stop_main(&mut self) {
        if let Some(run_loop_handle) = self.run_loop_handle.take() {
            run_loop_handle.abort();
        }

        self.shared_state.replacer.lock().await.close().await;
        *self.shared_state.last_outcome.lock().unwrap() = None;
        self.shared_state.status.set_down();

        // ping_receiver_handle task will shut down itself when Collector Worker drops the sender

        _ = self.ctx.stop_worker(self.collector_address.clone()).await;
    }

    /// Stop everything
    pub async fn stop(&mut self) {
        self.stop_additional().await;

        self.stop_main().await;
    }

    async fn send_ping(
        ctx: &Context,
        key: &str,
        collector_address: Address,
        pings: &mut Vec<Ping>,
        ping_route: Route,
    ) -> Result<()> {
        let ping = Ping::new();
        pings.push(ping);
        let ping_encoded = Encodable::encode(ping)?;

        let echo_route = route![ping_route.clone(), DefaultAddress::ECHO_SERVICE];
        trace! {
            key  = %key,
            addr = %ping_route,
            ping = %ping,
            "send ping"
        }

        let next = ping_route
            .next()
            .cloned()
            .unwrap_or(DefaultAddress::ECHO_SERVICE.into());

        if let Some(flow_control_id) = ctx
            .flow_controls()
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            ctx.flow_controls()
                .add_consumer(collector_address.clone(), &flow_control_id);
        }

        let local_message = LocalMessage::new()
            .with_onward_route(echo_route)
            .with_return_route(route![collector_address])
            .with_payload(ping_encoded);

        ctx.forward(local_message).await?;

        Ok(())
    }

    /// Continuously check the session.
    ///
    /// This method never returns. It will ping healthy session and
    /// trigger replacements if it's  unhealthy.
    async fn run_loop(
        ctx: Context,
        key: String,
        initial_connect_was_called: bool,
        collector_address: Address,
        shared_state: SharedState,
        ping_interval: Duration,
        retry_delay: Duration,
    ) {
        let mut first_creation = true;
        loop {
            trace!("check session");

            let mut pings = shared_state.sent_pings.lock().await;

            let status = shared_state.status.lock_clone();

            match status {
                StatusInternal::Up { ping_route } if pings.len() < MAX_FAILURES => {
                    match Self::send_ping(
                        &ctx,
                        &key,
                        collector_address.clone(),
                        &mut pings,
                        ping_route,
                    )
                    .await
                    {
                        Ok(_) => {
                            trace!(key = %key, "sent ping")
                        }
                        Err(err) => {
                            error!(key = %key, err = %err, "failed to send ping")
                        }
                    }

                    drop(pings);

                    sleep(ping_interval).await;
                }
                // The session is down, or we reached the maximum number of failures
                _ => {
                    let mut replacer = shared_state.replacer.lock().await;

                    if first_creation && !initial_connect_was_called {
                        debug!(key = %key, "session is down. starting");
                        first_creation = false;
                    } else {
                        warn!(key = %key, "session unresponsive. replacing");
                    }

                    if !first_creation && pings.len() > 0 {
                        replacer.on_session_down().await;
                    }

                    shared_state.status.set_down();
                    *shared_state.last_outcome.lock().unwrap() = None;
                    shared_state
                        .is_being_replaced
                        .store(true, Ordering::Relaxed);
                    pings.clear();
                    drop(pings);

                    match replacer.create().await {
                        Ok(replacer_outcome) => {
                            info!(key = %key, ping_route = %replacer_outcome.ping_route, "replacement is up");
                            if !first_creation {
                                replacer.on_session_replaced().await;
                            }

                            shared_state.status.set_up(replacer_outcome.ping_route);
                            shared_state
                                .is_being_replaced
                                .store(false, Ordering::Relaxed);
                            *shared_state.last_outcome.lock().unwrap() =
                                Some(replacer_outcome.kind.clone());
                        }
                        Err(err) => {
                            warn!(key = %key, err = %err, "replacing session failed");

                            shared_state
                                .is_being_replaced
                                .store(false, Ordering::Relaxed);

                            // Avoid retrying too often if it fails
                            sleep(retry_delay).await;
                        }
                    }
                }
            }
        }
    }

    /// Continuously check the session.
    ///
    /// This method never returns. It will ping healthy session and
    /// trigger replacements if it's  unhealthy.
    #[allow(clippy::too_many_arguments)]
    async fn run_loop_additional(
        ctx: Context,
        key: String,
        initial_connect_was_called: bool,
        shared_state: SharedState,
        enable_fallback: bool,
        additional_shared_state: AdditionalSharedState,
        additional_collector_address: Address,
        ping_interval: Duration,
        retry_delay: Duration,
    ) {
        let mut first_creation = true;

        // Start additional a little bit sooner, so that if both sessions are down, we have
        // a higher chance to first notice that for the main connection
        sleep(Duration::from_millis(100)).await;

        loop {
            trace!("check additional session");

            let mut pings = additional_shared_state.sent_pings.lock().await;

            let status = additional_shared_state.status.lock_clone();

            match status {
                StatusInternal::Up { ping_route } if pings.len() < MAX_FAILURES => {
                    match Self::send_ping(
                        &ctx,
                        &key,
                        additional_collector_address.clone(),
                        &mut pings,
                        ping_route,
                    )
                    .await
                    {
                        Ok(_) => {
                            trace!(key = %key, "sent additional ping")
                        }
                        Err(err) => {
                            error!(key = %key, err = %err, "failed to send additional ping")
                        }
                    }

                    drop(pings);

                    sleep(ping_interval).await;
                }
                _ => {
                    pings.clear();
                    drop(pings);

                    // We reached the maximum number of failures
                    additional_shared_state.status.set_down();

                    // We can't restart additional session until the main session is up
                    shared_state.status.wait_until_up().await;

                    if first_creation && !initial_connect_was_called {
                        info!(key = %key, "additional session is down. starting");
                        first_creation = false;
                    } else {
                        warn!(key = %key, "additional session unresponsive. replacing");
                    }

                    let mut replacer_lock = additional_shared_state.replacer.lock().await;
                    replacer_lock.close_additional(enable_fallback).await;
                    additional_shared_state
                        .is_being_replaced
                        .store(true, Ordering::Relaxed);
                    let res = replacer_lock.create_additional().await;
                    drop(replacer_lock);

                    match res {
                        Ok(ping_route) => {
                            info!(key = %key, ping_route = %ping_route, "replacement additional is up");

                            additional_shared_state.status.set_up(ping_route);
                            additional_shared_state
                                .is_being_replaced
                                .store(false, Ordering::Relaxed);
                        }
                        Err(err) => {
                            warn!(key = %key, err = %err, "replacing additional session failed");

                            additional_shared_state
                                .is_being_replaced
                                .store(false, Ordering::Relaxed);

                            // Avoid retrying too often if it fails
                            sleep(retry_delay).await;
                        }
                    }
                }
            }
        }
    }

    async fn wait_for_pings(
        key: String,
        mut pong_receiver: mpsc::Receiver<Ping>,
        pings: Arc<AsyncMutex<Vec<Ping>>>,
    ) {
        while let Some(ping) = pong_receiver.recv().await {
            let mut pings_guard = pings.lock().await;
            if pings_guard.contains(&ping) {
                trace!(%key, %ping, "recv pong");
                pings_guard.clear()
            }
        }
    }

    pub fn collector_address(&self) -> &Address {
        &self.collector_address
    }
}
