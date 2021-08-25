use crate::{
    protocols::{ParserFragment, ProtocolPayload},
    Address, Any, Context, Message, ProtocolId, Result, Route, Routed, TransportMessage, Worker,
};
use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};

/// A protocol exchanged between a stream consumer and stream producer
#[derive(Debug, Serialize, Deserialize)]
pub enum StreamWorkerCmd {
    /// Trigger a fetch event
    ///
    /// These events are fired from worker to _itself_ to create a
    /// delayed reactive response
    Fetch,
    /// Pull messages from the consumer's buffer
    Pull { num: usize },
}

impl StreamWorkerCmd {
    pub fn fetch() -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.fetch"), Self::Fetch)
    }

    /// Pull messages from the consumer's buffer
    ///
    /// When sending `Pull { num: 0 }` all available messages are
    /// pulled.  It is recommended to configure your stream consumer
    /// into ["forwarding mode"](crate::stream::Stream::recipient).
    pub fn pull(num: usize) -> ProtocolPayload {
        ProtocolPayload::new(ProtocolId::from("internal.stream.pull"), Self::Pull { num })
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
    _w: core::marker::PhantomData<W>,
}

impl<W, F> StreamCmdParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<StreamWorkerCmd>) -> Result<bool>,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _w: core::marker::PhantomData,
        }
    }
}

impl<W, F> ParserFragment<W> for StreamCmdParser<W, F>
where
    W: Worker,
    F: Fn(&mut W, &mut Context, Routed<StreamWorkerCmd>) -> Result<bool>,
{
    fn ids(&self) -> Vec<ProtocolId> {
        vec!["internal.stream.fetch", "internal.stream.pull"]
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
        let (addr, local_msg) = routed.dissolve();
        (&self.f)(state, ctx, Routed::new(cmd, addr, local_msg))
    }
}
