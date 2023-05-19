use core::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub(crate) enum Role {
    Initiator,
    Responder,
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
