use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::cloud::enroll::enrollment_token::{
    AuthenticateEnrollmentToken, EnrollmentToken, RequestEnrollmentToken,
};
use crate::cloud::Controller;

use ockam_core::api::{Reply, Request};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_identity::Attributes;
use ockam_node::Context;

const TARGET: &str = "ockam_api::cloud::enroll";

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[cbor(transparent)]
#[serde(transparent)]
pub struct Token(#[n(0)] pub String);

impl Token {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

#[async_trait]
trait Enroll {
    async fn generate_enrollment_token(
        &self,
        ctx: &Context,
        attributes: Attributes,
    ) -> Result<Reply<EnrollmentToken>>;

    async fn authenticate_enrollment_token(
        &self,
        ctx: &Context,
        enrollment_token: EnrollmentToken,
    ) -> Result<Reply<()>>;
}

#[async_trait]
impl Enroll for Controller {
    async fn generate_enrollment_token(
        &self,
        ctx: &Context,
        attributes: Attributes,
    ) -> Result<Reply<EnrollmentToken>> {
        trace!(target: TARGET, "generating tokens");
        let req = Request::post("v0/").body(RequestEnrollmentToken::new(attributes));
        self.0.ask(ctx, "projects", req).await
    }

    async fn authenticate_enrollment_token(
        &self,
        ctx: &Context,
        enrollment_token: EnrollmentToken,
    ) -> Result<Reply<()>> {
        let req = Request::post("v0/enroll").body(AuthenticateEnrollmentToken {
            token: enrollment_token.token,
        });
        trace!(target: TARGET, "authenticating token");
        self.0
            .tell(ctx, "enrollment_token_authenticator", req)
            .await
    }
}

pub mod auth0 {
    use super::*;

    // Req/Res types

    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct DeviceCode<'a> {
        pub device_code: Cow<'a, str>,
        pub user_code: Cow<'a, str>,
        pub verification_uri: Cow<'a, str>,
        pub verification_uri_complete: Cow<'a, str>,
        pub expires_in: usize,
        pub interval: usize,
    }

    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct AuthorizationCode {
        pub code: String,
    }

    impl AuthorizationCode {
        pub fn new(s: impl Into<String>) -> Self {
            Self { code: s.into() }
        }
    }

    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct TokensError<'a> {
        pub error: Cow<'a, str>,
        pub error_description: Cow<'a, str>,
    }

    #[derive(serde::Deserialize, Debug, Clone)]
    #[cfg_attr(test, derive(PartialEq, Eq))]
    pub struct OidcToken {
        pub token_type: TokenType,
        pub access_token: Token,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq)]
    #[cfg_attr(test, derive(Default))]
    pub struct UserInfo {
        pub sub: String,
        pub nickname: String,
        pub name: String,
        pub picture: String,
        pub updated_at: String,
        pub email: String,
        pub email_verified: bool,
    }

    #[derive(Encode, Decode, Debug)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateOidcToken {
        #[n(1)] pub token_type: TokenType,
        #[n(2)] pub access_token: Token,
    }

    impl AuthenticateOidcToken {
        pub fn new(token: OidcToken) -> Self {
            Self {
                token_type: token.token_type,
                access_token: token.access_token,
            }
        }
    }

    // Auxiliary types

    #[derive(serde::Deserialize, Encode, Decode, Debug, Clone)]
    #[cfg_attr(test, derive(PartialEq, Eq))]
    #[rustfmt::skip]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)] Bearer,
    }
}

pub mod enrollment_token {
    use serde::Serialize;

    use ockam::identity::Attributes;

    use super::*;

    // Main req/res types

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct RequestEnrollmentToken {
        #[b(1)] pub attributes: Attributes,
    }

    impl RequestEnrollmentToken {
        pub fn new(attributes: Attributes) -> Self {
            Self { attributes }
        }
    }

    #[derive(Encode, Decode, Serialize, Debug)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct EnrollmentToken {
        #[n(1)] pub token: Token,
    }

    impl EnrollmentToken {
        pub fn new(token: Token) -> Self {
            Self { token }
        }
    }

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateEnrollmentToken {
        #[n(1)] pub token: Token,
    }

    impl AuthenticateEnrollmentToken {
        pub fn new(token: EnrollmentToken) -> Self {
            Self { token: token.token }
        }
    }
}
