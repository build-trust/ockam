use core::sync::atomic::{AtomicBool, Ordering};
use log::info;
use ockam::{route, Address, Context};
use ockam_api::session::replacer::{
    AdditionalSessionReplacer, CurrentInletStatus, ReplacerOutcome, ReplacerOutputKind,
    SessionReplacer,
};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Any, Error, NeutralMessage, Result, Route, Routed, Worker};
use std::sync::atomic::AtomicU8;
use std::time::Duration;

pub struct MockEchoer {
    pub responsive: Arc<AtomicBool>,
    pub drop_every: Arc<AtomicU8>,

    drop_counter: u8,
}

impl MockEchoer {
    pub fn new() -> Self {
        Self {
            responsive: Arc::new(AtomicBool::new(true)),
            drop_every: Arc::new(AtomicU8::new(0)),

            drop_counter: 0,
        }
    }
}

#[ockam::worker]
impl Worker for MockEchoer {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if !self.responsive.load(Ordering::Relaxed) {
            info!("Drop message responsive=false");
            return Ok(());
        }

        let drop_every = self.drop_every.load(Ordering::Relaxed);
        if drop_every != 0 {
            self.drop_counter += 1;

            if self.drop_counter == drop_every {
                info!("Drop message drop_counter={}", drop_every);
                self.drop_counter = 0;
                return Ok(());
            }
        }

        ctx.send(
            msg.return_route().clone(),
            NeutralMessage::from(msg.into_payload()),
        )
        .await?;
        info!("Echo message back");

        Ok(())
    }
}

pub struct MockHop {
    pub responsive: Arc<AtomicBool>,
}

impl MockHop {
    pub fn new() -> Self {
        Self {
            responsive: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[ockam::worker]
impl Worker for MockHop {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if !self.responsive.load(Ordering::Relaxed) {
            info!("Drop Hop message, {}", ctx.primary_address());
            return Ok(());
        }

        info!("Forward Hop message {}", ctx.primary_address());

        let msg = msg.into_local_message();
        let msg = msg.pop_front_onward_route()?;

        ctx.forward(msg).await
    }
}

#[derive(Clone)]
pub struct MockReplacer {
    pub name: String,
    pub create_called: Arc<AtomicBool>,
    pub recreate_called: Arc<AtomicBool>,
    pub close_called: Arc<AtomicBool>,
    pub succeeds: Arc<AtomicBool>,
    pub ping_route: Route,
}

impl Default for MockReplacer {
    fn default() -> Self {
        Self::new("", route![])
    }
}

impl MockReplacer {
    pub fn new(name: &str, ping_route: Route) -> Self {
        Self {
            name: name.to_string(),
            create_called: Arc::new(AtomicBool::new(false)),
            recreate_called: Arc::new(AtomicBool::new(false)),
            close_called: Arc::new(AtomicBool::new(false)),
            succeeds: Arc::new(AtomicBool::new(true)),
            ping_route,
        }
    }

    async fn create_impl(&mut self) -> Result<()> {
        self.create_called.store(true, Ordering::Relaxed);

        info!("MockReplacer {} create called", self.name);
        tokio::time::sleep(Duration::from_millis(500)).await;

        if !self.succeeds.load(Ordering::Relaxed) {
            info!("MockReplacer {} create failed", self.name);
            return Err(Error::new(Origin::Api, Kind::Invalid, ""));
        }

        info!("MockReplacer {} create succeeded", self.name);

        Ok(())
    }

    async fn recreate_impl(&mut self) -> Result<()> {
        self.recreate_called.store(true, Ordering::Relaxed);

        info!("MockReplacer {} recreate called", self.name);

        self.close_impl();
        self.create_impl().await
    }

    fn close_impl(&mut self) {
        self.close_called.store(true, Ordering::Relaxed);

        info!("MockReplacer {} close called", self.name);
    }
}

#[async_trait]
impl SessionReplacer for MockReplacer {
    async fn create(&mut self) -> Result<ReplacerOutcome> {
        self.create_impl().await?;

        Ok(ReplacerOutcome {
            ping_route: self.ping_route.clone(),
            kind: ReplacerOutputKind::Inlet(CurrentInletStatus {
                route: route![],
                worker: Some(Address::from_string("echo")),
            }),
        })
    }

    async fn close(&mut self) {
        self.close_impl()
    }

    async fn on_session_down(&self) {
        info!("MockReplacer {} on_session_down called", self.name);
    }

    async fn on_session_replaced(&self) {
        info!("MockReplacer {} on_session_replaced called", self.name);
    }
}

#[async_trait]
impl AdditionalSessionReplacer for MockReplacer {
    async fn create_additional(&mut self) -> Result<Route> {
        self.create_impl().await?;

        Ok(self.ping_route.clone())
    }

    async fn close_additional(&mut self, _enable_fallback: bool) {
        self.close_impl()
    }
}
