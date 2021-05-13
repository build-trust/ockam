use crate::{
    error::OckamError, protocols::ProtocolPayload, Any, Message, ProtocolId, Result, Routed, Worker,
};
use std::{collections::BTreeMap, marker::PhantomData, ops::Deref, sync::Arc, sync::RwLock};

/// A parser for a protocol fragment
///
/// **If you are not a protocol author, you may want to use
/// [`UserParser`](UserParser) instead!**
///
/// Protocols are implemented as separate structures, wrapped in a
/// carrier type.  Because Rust can't have a function return different
/// types from a function, each protocol message (here called
/// "Fragment") needs to be handled by a separate parser.
pub trait ParserFragment<W>
where
    W: Worker,
{
    /// Return the set of `ProtocolID`s this parser can handle
    fn ids(&self) -> Vec<ProtocolId>;

    /// Parse an incoming message for a particular worker
    fn parse(&self, _state: &mut W, _msg: ProtocolPayload) -> Result<()> {
        Ok(())
    }
}

/// A user-closure to map protocol return values to worker state
pub type UserParser<W, T> = Box<dyn Fn(&mut W, T) + Send + Sync + 'static>;

/// An extensible protocol parser abstraction
///
/// ## The problem
///
/// In an Ockam worker system, a single worker can only ever accept
/// _one_ strong message type, defined via its associated type.  This
/// is very useful for input checking to a worker, but prevents it
/// from being able to asynchronously handling multiple message types,
/// and thus protocols.
///
/// The Ockam ProtocolParser exists to solve this problem.
///
/// ## How to use
///
/// Create a `ProtocolParser` and store it in your worker (as an
/// `Arc<ProtocolParser>).  During your workers initialise function
/// you should also initialise the protocol parser.  This is done by
/// mapping a [`ProtocolId`] to a [`Parser`].  For any Ockam-internal
/// protocol a Parser implementation is provided in the same module as
/// the basic structure definitions.
///
/// [`ProtocolId]: ockam_core::ProtocolId
/// [`ProtocolParselet]: ockam::protocol::Paser;
#[derive(Default)]
pub struct ProtocolParser<W: Worker>(Arc<ProtocolParserImpl<W>>);

impl<W: Worker> ProtocolParser<W> {
    /// Create a new `ProtocolParser`
    pub fn new() -> Self {
        Self(Arc::new(ProtocolParserImpl {
            map: Default::default(),
            _w: PhantomData,
        }))
    }

    /// Prepare the state of the parser
    ///
    /// This is required to get around mutable borrowing rules in the
    /// worker state, when passing the state to `parse()`.
    pub fn prepare(&self) -> Arc<ProtocolParserImpl<W>> {
        Arc::clone(&self.0)
    }
}

impl<W: Worker> Deref for ProtocolParser<W> {
    type Target = Arc<ProtocolParserImpl<W>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct ProtocolParserImpl<W: Worker> {
    map: RwLock<BTreeMap<ProtocolId, Arc<Box<dyn ParserFragment<W> + Send + Sync>>>>,
    _w: PhantomData<W>,
}

impl<W: Worker> ProtocolParserImpl<W> {
    /// Attach a new parser tree to this protocol parser
    pub fn attach<P>(self: &Arc<Self>, parser: P)
    where
        P: ParserFragment<W> + Send + Sync + 'static,
    {
        let p: Arc<Box<dyn ParserFragment<W> + Send + Sync>> = Arc::new(Box::new(parser));

        let mut map = self.map.write().unwrap();
        p.ids().into_iter().for_each(|pid| {
            map.insert(pid, Arc::clone(&p));
        });
    }

    /// Parse a message based on its protocol
    ///
    /// You may want to call [`prepare()`](Self::prepare) before
    /// calling this function.
    pub fn parse(self: Arc<Self>, w: &mut W, msg: Routed<Any>) -> Result<()> {
        let msg = msg.into_transport_message();

        // Parse message as a ProtocolPayload to grab the ProtocolId
        let proto_msg = ProtocolPayload::decode(&msg.payload).unwrap();
        let proto = ProtocolId::from_str(proto_msg.protocol.as_str());

        trace!("Parsing message for '{:?}' protocol", proto.as_str());

        // Get the protocol specific parser
        let map = self.map.read().unwrap();
        let parser = map.get(&proto).ok_or(OckamError::NoSuchParser)?;

        // Finally call the parser
        parser.parse(w, proto_msg)
    }
}
