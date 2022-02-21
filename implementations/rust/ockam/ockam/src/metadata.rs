use core::ops::{Deref, DerefMut};
use ockam_core::{
    compat::{collections::BTreeMap, string::String, vec::Vec},
    Address, Any, Decodable, Encodable, LocalMessage, Message, Result, Route, Routed,
    TransportMessage,
};
use serde::{Deserialize, Serialize};

/// A message metadata wrapper type
///
/// This message wraps around a well-typed Message type, with
/// additional metadata.  Metadata is split between the "scope" and
/// "generic" sections.
///
/// ## Scope metadata
///
/// This metadata is passed around in a particular metadata scope.
/// For example, a worker that adds some behaviour to message sending
/// may chose to embed "scope" metadata.  When wrapping this message
/// in another scope the previously scoped metadata becomes part of
/// the opaque `data` section.
///
/// Thus it is not possible to retrieve metadata from a different
/// nested scope!
///
/// ## Generic metadata
///
/// When creating an `OckamMessage` it's also possible to attach
/// generic metadata.  This data is passed around for every nested
/// scope and must be re-attached to the outest-most scope when
/// peeling a nested message stack.
#[derive(Clone, Message, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OckamMessage {
    /// Main data section of this message
    pub data: Vec<u8>,
    /// Metadata for this specific scope
    pub scope: Vec<Vec<u8>>,
    /// Metadata that is carried to the final recipient of the message
    pub generic: Option<Metadata>,
}

impl OckamMessage {
    pub fn new<M: Message>(msg: M) -> Result<Self> {
        Ok(Self {
            data: msg.encode()?,
            scope: vec![],
            generic: None,
        })
    }

    /// Create a new OckamMessage from an untyped Any message
    pub fn from_any(msg: Routed<Any>) -> Result<Self> {
        Ok(Self::decode(&msg.payload())?)
    }

    /// Create a new `OckamMessage` by nesting a previous one
    pub fn wrap(mut prev: Self) -> Result<Self> {
        let generic = core::mem::replace(&mut prev.generic, None);
        Ok(Self {
            data: prev.encode()?,
            scope: vec![],
            generic,
        })
    }

    /// Wrap this OckamMessage with a new `Routed` message type
    pub fn into_routed(
        self,
        msg_addr: Address,
        onward_route: Route,
        return_route: Route,
    ) -> Result<Routed<Self>> {
        let local = LocalMessage::new(
            TransportMessage::v1(onward_route, return_route, self.encode()?),
            vec![],
        );
        Ok(Routed::new(self, msg_addr, local))
    }

    /// Add some metadata to this scope
    pub fn scope_data(mut self, meta: Vec<u8>) -> Self {
        self.scope.push(meta);
        self
    }

    /// Add to the generic metadata section
    pub fn generic_data<S: Into<String>>(mut self, key: S, val: Vec<u8>) -> Self {
        if self.generic.is_none() {
            self.generic = Some(Metadata(BTreeMap::new()));
        }

        self.generic.as_mut().unwrap().insert(key.into(), val);
        self
    }

    /// Dissolve this outer layer of Message and reveal nested message
    ///
    /// Will throw a deserialisation error if the inner data is NOT an
    /// OckamMessage!
    pub fn peel(mut self) -> Result<Self> {
        let generic = core::mem::replace(&mut self.generic, None);
        let mut peeled = Self::decode(&self.data)?;
        peeled.generic = generic;
        Ok(peeled)
    }

    /// Decode the data section of this OckamMessage
    pub fn data<M: Message>(&self) -> Result<M> {
        Ok(M::decode(&self.data)?)
    }
}

/// An encoding for message metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata(BTreeMap<String, Vec<u8>>);

impl Deref for Metadata {
    type Target = BTreeMap<String, Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Metadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// This test emulates the message flow for a very simple pipe
/// metadata message.  At the core we have some piece of data, which
/// gets wrapped in a bespoke message type.  This message then is
/// attached with scope metadata to indicate the message index.
#[test]
fn nest_metadata() {
    #[derive(Serialize, Deserialize, Message, PartialEq, Debug, Clone)]
    struct FakePipeMsg {
        vec: Vec<u8>,
    }

    let base_msg = FakePipeMsg {
        vec: vec![1, 2, 3, 4, 5, 6, 7, 8],
    };

    // The base message type with a msg_type generic metadata field
    let ockam_msg1 = OckamMessage::new(base_msg.clone())
        .unwrap()
        .generic_data("msg_type", "pipemsg".as_bytes().into());

    // Wrap this message in another scope which adds a message index
    // to the scope metadata section.  `vec![1]` is our index here but
    // in reality this should be a properly encoded type!
    let ockam_msg2 = OckamMessage::wrap(ockam_msg1).unwrap().scope_data(vec![1]);

    /////////// On the other side of a transport

    let msg_type = ockam_msg2
        .generic
        .as_ref()
        .unwrap()
        .get("msg_type")
        .unwrap();
    assert_eq!(msg_type, "pipemsg".as_bytes());

    let index = ockam_msg2.scope.get(0).unwrap();
    assert_eq!(index, &vec![1]);

    // Then we peel the previous message type
    let ockam_msg1 = ockam_msg2.peel().unwrap();
    let base_msg_other_side = FakePipeMsg::decode(&ockam_msg1.data).unwrap();

    assert_eq!(base_msg, base_msg_other_side);
}
