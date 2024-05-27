use crate::IdentityError;
use core::fmt::{Display, Formatter};
use ockam_core::Error;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Role {
    Initiator,
    Responder,
}

impl Role {
    pub const INITIATOR: &'static str = "initiator";
    pub const RESPONDER: &'static str = "responder";
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            if self.is_initiator() {
                "Initiator"
            } else {
                "Responder"
            }
        )
    }
}

impl TryFrom<&str> for Role {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            Self::INITIATOR => Ok(Self::Initiator),
            Self::RESPONDER => Ok(Self::Responder),
            _ => Err(IdentityError::UnknownRole)?,
        }
    }
}

impl Role {
    pub fn is_initiator(&self) -> bool {
        match self {
            Self::Initiator => true,
            Self::Responder => false,
        }
    }

    pub fn str(&self) -> &'static str {
        match self {
            Self::Initiator => Self::INITIATOR,
            Self::Responder => Self::RESPONDER,
        }
    }
}
