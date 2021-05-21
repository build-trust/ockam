use crate::{
    protocols::{ParserFragment, ProtocolPayload},
    Address, Any, Context, Message, ProtocolId, Result, Route, Routed, TransportMessage, Worker,
};
use serde::{Deserialize, Serialize};

/// A protocol exchanged between a stream consumer and stream producer
#[derive(Debug, Serialize, Deserialize)]
pub enum StreamWorkerCmd {
    /// Trigger a fetch event
    ///
    /// These events are fired from worker to _itself_ to create a
    /// delayed reactive response
    Fetch,
    /// Initialise the peer route for the producer
    Init { peer: Route },
    /// Pull messages from the consumer's buffer
    Pull { num: usize },
    /// A forwarded message envelope
    Forward(TransportMessage),
}

impl StreamWorkerCmd {
    pub fn fetch() -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.fetch"), Self::Fetch)
    }

    pub fn init(peer: Route) -> ProtocolPayload {
        ProtocolPayload::new(
            ProtocolId::from("internal.stream.init"),
            Self::Init { peer },
        )
    }

    /// Pull messages from the consumer's buffer
    ///
    /// When sending `Pull { num: 0 }` all available messages are
    /// pulled.  It is recommended to configure your stream consumer
    /// into ["forwarding mode"](crate::stream::Stream::recipient).
    pub fn pull(num: usize) -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.pull"), Self::Pull { num })
    }

    pub fn fwd(msg: TransportMessage) -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.fwd"), Self::Forward(msg))
    }
}

pub(crate) fn parse(msg: &TransportMessage) -> Option<StreamWorkerCmd> {
    StreamWorkerCmd::decode(&msg.payload).ok()
}

pub struct StreamCmdParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<StreamWorkerCmd>) -> Result<bool>,
{
    f: F,
    _w: std::marker::PhantomData<W>,
}

impl<W, F> StreamCmdParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<StreamWorkerCmd>) -> Result<bool>,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _w: std::marker::PhantomData,
        }
    }
}

impl<W, F> ParserFragment<W> for StreamCmdParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<StreamWorkerCmd>) -> Result<bool>,
{
    fn ids(&self) -> Vec<ProtocolId> {
        vec![
            "internal.stream.fetch",
            "internal.stream.init",
            "internal.stream.pull",
            "internal.stream.fwd",
        ]
        .into_iter()
        .map(Into::into)
        .collect()
    }

    fn parse(
        &self,
        state: &mut W,
        ctx: &mut Context,
        routed: &Routed<Any>,
        msg: ProtocolPayload,
    ) -> Result<bool> {
        let cmd = StreamWorkerCmd::decode(&msg.data)?;
        let (addr, trans) = routed.dissolve();
        (&self.f)(state, ctx, Routed::v1(cmd, addr, trans))
    }
}
