use ockam_core::CowStr;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use std::time::Duration;

pub struct Connection<'a, T> {
    pub transport: &'a T,
    pub ctx: &'a Context,
    pub addr: &'a MultiAddr,
    pub identity_name: Option<CowStr<'a>>,
    pub credential_name: Option<CowStr<'a>>,
    pub authorized_identities: Option<IdentityIdentifier>,
    pub timeout: Option<Duration>,
}

impl<'a> Connection<'a, TcpTransport> {
    pub fn new(transport: &'a TcpTransport, ctx: &'a Context, addr: &'a MultiAddr) -> Self {
        Self {
            transport,
            ctx,
            addr,
            identity_name: None,
            credential_name: None,
            authorized_identities: None,
            timeout: None,
        }
    }

    pub fn with_identity_name<T: Into<Option<CowStr<'a>>>>(mut self, identity_name: T) -> Self {
        self.identity_name = identity_name.into();
        self
    }

    #[allow(unused)]
    pub fn with_credential_name<T: Into<Option<CowStr<'a>>>>(mut self, credential_name: T) -> Self {
        self.credential_name = credential_name.into();
        self
    }

    pub fn with_authorized_identities<T: Into<Option<IdentityIdentifier>>>(
        mut self,
        authorized_identities: T,
    ) -> Self {
        self.authorized_identities = authorized_identities.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
