use minicbor::{Decode, Encode};
use ockam_core::CowStr;
use serde::{Deserialize, Serialize};

// ======= CREATE TOKEN REQUEST AND RESPONSE =======
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct CreateTokenRequest<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub description: Option<CowStr<'a>>,

    #[serde(borrow)]
    #[cbor(b(2))]
    pub status: Option<CowStr<'a>>,

    #[serde(borrow)]
    #[cbor(b(3))]
    pub user_id: Option<CowStr<'a>>,
}

impl<'a> CreateTokenRequest<'a> {
    pub fn new<S: Into<CowStr<'a>>>(
        description: Option<S>,
        status: Option<S>,
        user_id: Option<S>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            description: description.map(|d| d.into()),
            status: status.map(|s| s.into()),
            user_id: user_id.map(|uid| uid.into()),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct CreateTokenResponse<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub token: Token<'a>,
}

// ======= LIST TOKEN REQUEST AND RESPONSE =======

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct ListTokensRequest<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub user: Option<CowStr<'a>>,

    #[serde(borrow)]
    #[cbor(b(2))]
    pub user_id: Option<CowStr<'a>>,
}

impl<'a> ListTokensRequest<'a> {
    pub fn new<S: Into<CowStr<'a>>>(user: Option<S>, user_id: Option<S>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            user: user.map(|u| u.into()),
            user_id: user_id.map(|uid| uid.into()),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct ListTokensResponse<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub tokens: Vec<Token<'a>>,
}

// ======= SHOW TOKEN REQUEST AND RESPONSE =======

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct ShowTokenRequest<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub token_id: CowStr<'a>,
}

impl<'a> ShowTokenRequest<'a> {
    pub fn new<S: Into<CowStr<'a>>>(token_id: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            token_id: token_id.into(),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct ShowTokenResponse<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub token: Token<'a>,
}

// ======== REVOKE TOKEN STRUCT =========
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct RevokeTokenRequest<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip)]
    #[cbor(n(0))]
    pub tag: TypeTag<000000>, // TODO: Determine correct value for this type

    #[serde(borrow)]
    #[cbor(b(1))]
    pub token_id: CowStr<'a>,
}

impl<'a> RevokeTokenRequest<'a> {
    pub fn new<S: Into<CowStr<'a>>>(token_id: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            token_id: token_id.into(),
        }
    }
}

// ======= TOKEN STRUCT =======
#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct Token<'a> {
    #[serde(borrow)]
    #[cbor(b(0))]
    pub id: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(1))]
    pub org: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(2))]
    pub org_id: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(3))]
    pub status: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(4))]
    pub token: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(5))]
    pub created_at: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(6))]
    pub updated_at: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(7))]
    pub user: CowStr<'a>,

    #[serde(borrow)]
    #[cbor(b(8))]
    pub user_id: CowStr<'a>,
}
