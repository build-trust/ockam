#![allow(unused)]

use crate::protocols::stream::responses::StreamMessage;

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
    pub async fn next<T: Message>(&mut self) -> Result<T> {
        let stream_msg = self
            .ctx
            .receive_block::<StreamMessage>()
            .await
            .unwrap()
            .take()
            .body();

        let transport = TransportMessage::decode(&stream_msg.data).unwrap();
        T::decode(&transport.payload)

        // self.ctx
        //     // We receive a StreamMessage from the stream
        //     .receive_block::<StreamMessage>()
        //     .await
        //     .map(|c| c.take())
        //     .and_then(|proto| {
        //         let payload = proto.payload();
        //         let (addr, trans) = proto.dissolve();

        //         // Which we first map into the underlying `TransportMessage`
        //         TransportMessage::decode(payload)
        //             // and then into the actual type
        //             .and_then(|trans| T::decode(&trans.payload))
        //             // Which we also wrap in a `Routed`
        //             .map(|t| Routed::v1(t, addr, trans))
        //     })
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
    pub fn with_interval<D: Into<Duration>>(self, duration: D) -> Self {
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
    pub fn with_recipient<A: Into<Address>>(self, addr: A) -> Self {
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

        let rx_rx = Address::random(0);

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
                    rx_rx.clone(),
                ),
            )
            .await?;

        // Create and start a new stream producer
        self.ctx
            .start_worker(tx.clone(), StreamProducer::new())
            .await?;

        // Return a sender and receiver address
        Ok((
            SenderAddress { inner: tx },
            ReceiverAddress {
                inner: rx,
                ctx: self.ctx.new_context(rx_rx).await?,
            },
        ))
    }
}
