use ockam_core::CowStr;
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::time::Duration;

pub struct Connection<'a> {
    pub ctx: &'a Context,
    pub addr: &'a MultiAddr,
    pub identity_name: Option<CowStr<'a>>,
    pub credential_name: Option<CowStr<'a>>,
    pub authorized_identities: Option<Vec<IdentityIdentifier>>,
    pub timeout: Option<Duration>,
}

impl<'a> Connection<'a> {
    pub fn new(ctx: &'a Context, addr: &'a MultiAddr) -> Self {
        Self {
            ctx,
            addr,
            identity_name: None,
            credential_name: None,
            authorized_identities: None,
            timeout: None,
        }
    }

    #[allow(unused)]
    pub fn with_credential_name<T: Into<Option<CowStr<'a>>>>(mut self, credential_name: T) -> Self {
        self.credential_name = credential_name.into();
        self
    }

    pub fn with_authorized_identity<T: Into<Option<IdentityIdentifier>>>(
        mut self,
        authorized_identity: T,
    ) -> Self {
        self.authorized_identities = authorized_identity.into().map(|x| vec![x]);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}
