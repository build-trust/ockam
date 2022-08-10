use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body when instructing a node to start a Vault service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVaultServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9798850>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartVaultServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Identity service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartIdentityServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6129106>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartIdentityServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Authenticated service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatedServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5179596>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartAuthenticatedServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Uppercase service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartUppercaseServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8177400>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartUppercaseServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Echoer service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartEchoerServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7636656>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartEchoerServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatorRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4724285>,
    #[b(1)] addr: Cow<'a, str>,
    #[n(2)] typ: AuthenticatorType
}

#[derive(Debug, Clone, Copy, Decode, Encode)]
#[cbor(index_only)]
pub enum AuthenticatorType {
    #[n(0)]
    Direct,
}

impl<'a> StartAuthenticatorRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>, typ: AuthenticatorType) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
            typ,
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }

    pub fn typ(&self) -> AuthenticatorType {
        self.typ
    }
}
