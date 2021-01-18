use crate::profile::change_event::ChangeEvent;
use crate::profile::EventId;

pub enum SignatureType {
    SelfSign,
    Previous,
}

pub struct Signature {
    stype: SignatureType, // Replace with enum
    data: [u8; 64],
}

impl Signature {
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    pub fn data(&self) -> &[u8; 64] {
        &self.data
    }
}

impl Signature {
    pub fn new(stype: SignatureType, data: [u8; 64]) -> Self {
        Signature { stype, data }
    }
}

pub struct SignedChangeEvent {
    version: u8,
    identifier: EventId,
    binary: Vec<u8>, // May be removed if needed, but may be useful
    change_event: ChangeEvent,
    signature: Vec<Signature>,
}

impl SignedChangeEvent {
    pub(crate) fn new(
        version: u8,
        identifier: EventId,
        binary: Vec<u8>,
        change_event: ChangeEvent,
        signature: Vec<Signature>,
    ) -> Self {
        SignedChangeEvent {
            version,
            identifier,
            binary,
            change_event,
            signature,
        }
    }
}

impl SignedChangeEvent {
    pub fn version(&self) -> u8 {
        self.version
    }
    pub fn identifier(&self) -> &EventId {
        &self.identifier
    }
    pub fn binary(&self) -> &[u8] {
        &self.binary
    }
    pub fn change_event(&self) -> &ChangeEvent {
        &self.change_event
    }
    pub fn signature(&self) -> &[Signature] {
        &self.signature
    }
}
