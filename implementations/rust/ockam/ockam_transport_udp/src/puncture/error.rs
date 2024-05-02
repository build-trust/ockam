use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// UDP Puncture errors
#[derive(Clone, Copy, Debug)]
pub enum PunctureError {
    /// Unable to reach the Rendezvous Service
    RendezvousServiceNotFound,
    /// Puncture to peer not open
    PunctureNotOpen,
    /// Internal error, possibly a bug
    Internal,
    /// We received an unexpected message type
    NegotiationInvalidMessageType,
    /// We received an unexpected message type from Rendezvous service
    RendezvousResponseInvalidMessageType,
}

impl ockam_core::compat::error::Error for PunctureError {}
impl core::fmt::Display for PunctureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl From<PunctureError> for Error {
    #[track_caller]
    fn from(err: PunctureError) -> Self {
        use PunctureError::*;
        let kind = match err {
            RendezvousServiceNotFound | PunctureNotOpen => Kind::NotFound,
            Internal => Kind::Internal,
            NegotiationInvalidMessageType | RendezvousResponseInvalidMessageType => Kind::Invalid,
        };
        Error::new(Origin::Other, kind, err)
    }
}
