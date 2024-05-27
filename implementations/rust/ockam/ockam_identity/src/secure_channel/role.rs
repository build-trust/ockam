use core::fmt::{Display, Formatter};
use ockam_core::Error;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Role {
    Initiator,
    Responder,
}

impl From<Role> for u8 {
    fn from(value: Role) -> Self {
        match value {
            Role::Initiator => 0,
            Role::Responder => 1,
        }
    }
}

impl TryFrom<u8> for Role {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Role::Initiator),
            1 => Ok(Role::Responder),
            _ => panic!(), // FIXME
        }
    }
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

impl Role {
    pub fn is_initiator(&self) -> bool {
        match self {
            Role::Initiator => true,
            Role::Responder => false,
        }
    }

    pub fn str(&self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Responder => "responder",
        }
    }
}
