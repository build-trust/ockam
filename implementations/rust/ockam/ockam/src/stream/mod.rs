#![allow(unused)]

use crate::protocols::stream::responses::StreamMessage;
use rand::{distributions::Standard, prelude::Distribution, Rng};

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
    pub async fn next<T: Message>(&mut self) -> Result<Routed<T>> {
        let routed = self.ctx.receive_block::<StreamMessage>().await?.take();
        let stream_msg = routed.as_body();
        let (addr, trans) = routed.dissolve();

        let transport = TransportMessage::decode(&stream_msg.data).unwrap();
        T::decode(&transport.payload).map(|t| Routed::v1(t, addr, trans))
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

    /// Connect to a bi-directional stream by remote and stream pair
    ///
    /// When using the stream protocol for bi-directional
    /// communication a sending and receiving stream name is required.
    /// These two identifiers MUST be known between nodes that wish to
    /// exchange messages.
    ///
    /// The `peer` parameter is the route to a remote which hosts a
    /// `stream_service` and `stream_index_service`, such as
    /// hub.ockam.io.
    ///
    /// Streams that do not already exists will be created, and
    /// existing stream identifiers will automatically be re-used.
    pub async fn connect<R, S>(
        &self,
        peer: R,
        tx_name: S,
        rx_name: S,
    ) -> Result<(SenderAddress, ReceiverAddress)>
    where
        R: Into<Route>,
        S: Into<String>,
    {
        let peer = peer.into();
        let tx_name = tx_name.into();
        let rx_name = rx_name.into();

        // Generate two new random addresses
        let rx = Address::random(0);
        let tx = Address::random(0);

        let rx_rx = Address::random(0);

        // Generate a random client_id
        // TODO: there should be an API endpoint where users get to choose the client_id
        let client_id = {
            let random: [u8; 16] = rand::thread_rng().gen();
            hex::encode(random)
        };

        // Create and start a new stream consumer
        self.ctx
            .start_worker(
                rx.clone(),
                StreamConsumer::new(
                    client_id,
                    peer.clone(),
                    Some(tx.clone()),
                    rx_name.clone(),
                    self.interval.clone(),
                    self.recipient.clone(),
                    rx_rx.clone(),
                ),
            )
            .await?;

        // Create and start a new stream producer
        self.ctx
            .start_worker(tx.clone(), StreamProducer::new(tx_name.clone(), peer))
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
