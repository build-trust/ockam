#![allow(unused)]

use crate::protocols::stream::responses::StreamMessage;
use ockam_core::compat::string::String;
use ockam_core::RouteBuilder;

#[cfg(feature = "unsafe_random")]
use ockam_core::compat::rand::{self, distributions::Standard, prelude::Distribution, Rng};
#[cfg(not(feature = "unsafe_random"))]
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
use core::{ops::Deref, time::Duration};
use serde::{Deserialize, Serialize};

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
    forwarding_address: Option<Address>,
    stream_service: String,
    index_service: String,
    client_id: Option<String>,
}

/// A simple address wrapper for stream workers
///
/// This type can be used as any other address, while also exposing
/// the name of the stream it is associated with.
pub struct SenderAddress {
    inner: Address,
}

impl SenderAddress {
    /// Create a new route from this sender address
    pub fn to_route(&self) -> RouteBuilder {
        Route::new().append(self.inner.clone())
    }
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
        let (addr, local_msg) = routed.dissolve();

        let transport = TransportMessage::decode(&stream_msg.data).unwrap();
        T::decode(&transport.payload).map(|t| Routed::new(t, addr, local_msg))
    }
}

impl Stream {
    /// Create a new Ockam stream controller
    ///
    /// By default, the created stream will poll for new messages
    /// every 250 milliseconds.
    pub fn new(ctx: &Context) -> Result<Self> {
        block_future(&ctx.runtime(), async {
            ctx.new_context(Address::random(16)).await.map(|ctx| Self {
                ctx,
                interval: Duration::from_millis(250),
                forwarding_address: None,
                stream_service: "stream".into(),
                index_service: "stream_index".into(),
                client_id: None,
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

    /// Specify the stream service running on the remote
    pub fn stream_service<S: Into<String>>(self, serv: S) -> Self {
        Self {
            stream_service: serv.into(),
            ..self
        }
    }

    /// Specify the index service running on the remote
    pub fn index_service<S: Into<String>>(self, serv: S) -> Self {
        Self {
            index_service: serv.into(),
            ..self
        }
    }

    /// Specify the client_id for the stream consumer
    ///
    /// When setting up a stream without calling this function
    /// a random client id will be assigned.
    pub fn client_id<S: Into<String>>(self, client_id: S) -> Self {
        Self {
            client_id: Some(client_id.into()),
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
            forwarding_address: Some(addr.into()),
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
    /// The `route` parameter is the route to a remote which hosts a
    /// `stream_service` and `stream_index_service`, such as
    /// hub.ockam.io.
    ///
    /// Streams that do not already exists will be created, and
    /// existing stream identifiers will automatically be re-used.
    pub async fn connect<R, S>(
        &self,
        route: R,
        sender_name: S,
        receiver_name: S,
    ) -> Result<(SenderAddress, ReceiverAddress)>
    where
        R: Into<Route>,
        S: Into<String>,
    {
        let route = route.into();
        let sender_name = sender_name.into();
        let receiver_name = receiver_name.into();

        // Generate two new random addresses
        let receiver_address = Address::random(0);
        let sender_address = Address::random(0);

        let receiver_rx = Address::random(0);

        // Generate a random client_id if one has not been provided
        let client_id = match self.client_id.clone() {
            Some(client_id) => client_id,
            None => {
                let random: [u8; 16] = rand::thread_rng().gen();
                hex::encode(random)
            }
        };

        // Create and start a new stream consumer
        self.ctx
            .start_worker(
                receiver_address.clone(),
                StreamConsumer::new(
                    client_id,
                    route.clone(),
                    Some(sender_address.clone()),
                    receiver_name.clone(),
                    self.interval.clone(),
                    self.forwarding_address.clone(),
                    receiver_rx.clone(),
                    self.stream_service.clone(),
                    self.index_service.clone(),
                ),
            )
            .await?;

        // Create and start a new stream producer
        self.ctx
            .start_worker(
                sender_address.clone(),
                StreamProducer::new(sender_name.clone(), route, self.stream_service.clone()),
            )
            .await?;

        // Return a sender and receiver address
        Ok((
            SenderAddress {
                inner: sender_address,
            },
            ReceiverAddress {
                inner: receiver_address,
                ctx: self.ctx.new_context(receiver_rx).await?,
            },
        ))
    }
}
