use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// UDP NAT Hole Puncher errors
#[derive(Clone, Copy, Debug)]
pub enum PunchError {
    /// Unable to reach the Rendezvous Service
    RendezvousServiceNotFound,

    /// Hole to peer not open
    HoleNotOpen,

    /// Internal error, possibly a bug
    Internal,
}

impl ockam_core::compat::error::Error for PunchError {}
impl core::fmt::Display for PunchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl From<PunchError> for Error {
    #[track_caller]
    fn from(err: PunchError) -> Self {
        use PunchError::*;
        let kind = match err {
            RendezvousServiceNotFound | HoleNotOpen => Kind::NotFound,
            Internal => Kind::Internal,
        };
        Error::new(Origin::Other, kind, err)
    }
}
