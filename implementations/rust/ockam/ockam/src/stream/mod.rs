#![allow(unused)]

mod cmd;
pub use cmd::{StreamCmdParser, StreamWorkerCmd};

mod consumer;
use consumer::StreamConsumer;

mod producer;
use producer::StreamProducer;

use crate::{
    block_future,
    protocols::{
        stream::{requests::*, responses::*},
        ProtocolParser,
    },
    Address, Any, Context, DelayedEvent, Error, Message, Result, Route, Routed, TransportMessage,
    Worker,
};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, time::Duration};

/// Ockam stream protocol controller
///
/// Each stream has a sending and consuming worker (publisher and
/// consumer) that are created and managed on the fly by this
/// abstraction.
///
///
pub struct Stream {
    ctx: Context,
    interval: Duration,
    recipient: Option<Address>,
}

/// A simple address wrapper for stream workers
///
/// This type can be used as any other address, while also exposing
/// the name of the stream it is associated with.
pub struct SenderAddress {
    inner: Address,
}

impl Deref for SenderAddress {
    type Target = Address;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<SenderAddress> for Route {
    fn from(addr: SenderAddress) -> Self {
        Route::new().append(addr.inner.clone()).into()
    }
}

pub struct ReceiverAddress {
    ctx: Context,
    inner: Address,
}

impl ReceiverAddress {
    /// Wait for the next message received by the stream consumer
    pub async fn next<T: Message>(&self) -> T {
        todo!()
    }
}

impl Stream {
    /// Create a new Ockam stream controller
    ///
    /// The created stream will poll for new messages every second
    pub fn new(ctx: &Context) -> Result<Self> {
        block_future(&ctx.runtime(), async {
            ctx.new_context(Address::random(16)).await.map(|ctx| Self {
                ctx,
                interval: Duration::from_secs(10),
                recipient: None,
            })
        })
    }

    /// Customize the polling interval for the stream consumer
    pub fn interval<D: Into<Duration>>(self, duration: D) -> Self {
        Self {
            interval: duration.into(),
            ..self
        }
    }

    /// Specify an address to forward incoming messages to
    ///
    /// When setting up a stream without calling this function
    /// messages will be buffered by the StreamConsumer and must be
    /// polled via the [`StreamWorkerCmd`]().
    pub fn recipient<A: Into<Address>>(self, addr: A) -> Self {
        Self {
            recipient: Some(addr.into()),
            ..self
        }
    }

    /// Connect to a stream by name and remote route
    ///
    /// If the stream does not already exist, or no name was provided,
    /// a new stream will be generated.
    pub async fn connect<R, S>(&self, peer: R, name: S) -> Result<(SenderAddress, ReceiverAddress)>
    where
        R: Into<Route>,
        S: Into<Option<String>>,
    {
        let peer = peer.into();
        let name = name.into();

        // Generate two new random addresses
        let rx = Address::random(0);
        let tx = Address::random(0);

        // Create and start a new stream consumer
        self.ctx
            .start_worker(
                rx.clone(),
                StreamConsumer::new(
                    peer.clone(),
                    tx.clone(),
                    name.clone(),
                    self.interval.clone(),
                    self.recipient.clone(),
                ),
            )
            .await?;

        // Create and start a new stream producer

        // Return a sender and receiver address
        Ok((
            SenderAddress { inner: tx },
            ReceiverAddress {
                inner: rx,
                ctx: self.ctx.new_context(Address::random(16)).await?,
            },
        ))
    }
}
