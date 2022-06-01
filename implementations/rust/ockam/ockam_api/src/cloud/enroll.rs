use std::borrow::Cow;

use minicbor::{Decode, Decoder, Encode};

use ockam_core::{self, async_trait};

use crate::cloud::{error, response, MessagingClient};
#[cfg(feature = "tag")]
use crate::TypeTag;
use crate::{Request, Status};

pub enum Authenticator {
    Auth0,
    EnrollmentToken,
}

impl core::fmt::Display for Authenticator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Authenticator::Auth0 => "Auth0".fmt(f),
            Authenticator::EnrollmentToken => "EnrollmentToken".fmt(f),
        }
    }
}

#[async_trait::async_trait]
pub trait TokenProvider<'a> {
    type T;

    async fn token(&mut self, identity: &Identity<'a>) -> ockam_core::Result<Self::T>;
}

#[async_trait::async_trait]
pub trait TokenAuthenticatorService {
    async fn authenticate<'a>(&mut self, body: AuthorizedToken<'a>) -> ockam_core::Result<()>;
}

#[derive(Encode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[cbor(map)]
pub struct RequestEnrollment<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    pub tag: TypeTag<5136510>,
    #[n(1)]
    pub identity: Identity<'a>,
    #[n(2)]
    pub token: Token<'a>,
}

impl<'a> RequestEnrollment<'a> {
    pub fn new<I: Into<Identity<'a>>>(identity: I, token: Token<'a>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            token,
        }
    }
}

#[derive(serde::Deserialize, Encode, Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Decode))]
#[cbor(transparent)]
pub struct Identity<'a>(#[n(0)] Cow<'a, str>);

impl<'a> From<&'static str> for Identity<'a> {
    fn from(v: &'static str) -> Self {
        Self(v.into())
    }
}

impl<'a> From<String> for Identity<'a> {
    fn from(v: String) -> Self {
        Self(v.into())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Encode, Decode, Debug)]
#[cfg_attr(test, derive(PartialEq, Clone))]
#[cbor(transparent)]
pub struct Token<'a>(#[n(0)] pub Cow<'a, str>);

pub enum AuthorizedToken<'a> {
    Auth0(auth0::AuthorizedAuth0Token<'a>),
    EnrollmentToken(enrollment_token::AuthorizedEnrollmentToken<'a>),
}

#[async_trait::async_trait]
impl TokenAuthenticatorService for MessagingClient {
    async fn authenticate<'a>(&mut self, body: AuthorizedToken<'a>) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::enroll::authenticate";
        let label = "authenticate";

        match body {
            AuthorizedToken::Auth0(body) => {
                let route = self.route.modify().append("auth0_authenticator").into();
                let req = Request::post("v0/enroll").body(body);
                self.buf = self.request(target, label, route, &req).await?;
            }
            AuthorizedToken::EnrollmentToken(body) => {
                let route = self
                    .route
                    .modify()
                    .append("enrollment_token_authenticator")
                    .into();
                let req = Request::post("v0/enroll").body(body);
                self.buf = self.request(target, label, route, &req).await?;
            }
        };

        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }
}

pub mod auth0 {
    use super::*;

    pub const DOMAIN: &str = "dev-w5hdnpc2.us.auth0.com";
    pub const CLIENT_ID: &str = "sGyXBwQfU6fjfW1gopphdV9vCLec060b";
    pub const API_AUDIENCE: &str = "https://dev-w5hdnpc2.us.auth0.com/api/v2/";
    pub const SCOPES: &str = "profile openid email";

    // Req/Res types

    #[derive(serde::Deserialize, Debug, PartialEq)]
    pub struct DeviceCode<'a> {
        pub device_code: Cow<'a, str>,
        pub user_code: Cow<'a, str>,
        pub verification_uri: Cow<'a, str>,
        pub verification_uri_complete: Cow<'a, str>,
        pub expires_in: usize,
        pub interval: usize,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    pub struct TokensError<'a> {
        pub error: Cow<'a, str>,
        pub error_description: Cow<'a, str>,
    }

    #[derive(serde::Deserialize, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone))]
    pub struct Auth0Token<'a> {
        pub token_type: TokenType,
        pub access_token: Token<'a>,
    }

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[cbor(map)]
    pub struct AuthorizedAuth0Token<'a> {
        #[cfg(feature = "tag")]
        #[n(0)]
        pub tag: TypeTag<1058055>,
        #[n(1)]
        pub identity: Identity<'a>,
        #[n(2)]
        pub token_type: TokenType,
        #[n(3)]
        pub access_token: Token<'a>,
    }

    impl<'a> AuthorizedAuth0Token<'a> {
        pub fn new(identity: Identity<'a>, token: Auth0Token<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                identity,
                token_type: token.token_type,
                access_token: token.access_token,
            }
        }
    }

    // Auxiliary types

    #[derive(serde::Deserialize, Encode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Decode, Clone))]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)]
        Bearer,
    }
}

pub(crate) mod enrollment_token {
    use super::*;

    // Main req/res types

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[cbor(map)]
    pub struct RequestEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)]
        pub tag: TypeTag<8560526>,
        #[n(1)]
        pub identity: Identity<'a>,
        #[n(2)]
        pub attributes: Vec<Attribute<'a>>,
    }

    impl<'a> RequestEnrollmentToken<'a> {
        pub fn new<I: Into<Identity<'a>>>(identity: I, attributes: &[Attribute<'a>]) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                identity: identity.into(),
                attributes: attributes.to_vec(),
            }
        }
    }

    #[derive(Decode, Debug)]
    #[cfg_attr(test, derive(Encode, Clone))]
    #[cbor(map)]
    pub struct EnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)]
        pub tag: TypeTag<8932763>,
        #[n(1)]
        pub token: Token<'a>,
    }

    impl<'a> EnrollmentToken<'a> {
        pub fn new(token: Token<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token,
            }
        }
    }
    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[cbor(map)]
    pub struct AuthorizedEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)]
        pub tag: TypeTag<9463780>,
        #[n(1)]
        pub identity: Identity<'a>,
        #[n(2)]
        pub token: Token<'a>,
    }

    impl<'a> AuthorizedEnrollmentToken<'a> {
        pub fn new(identity: Identity<'a>, token: EnrollmentToken<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                identity,
                token: token.token,
            }
        }
    }

    // Auxiliary types

    #[derive(serde::Deserialize, Encode, Debug, Clone)]
    #[cfg_attr(test, derive(PartialEq, Decode))]
    #[cbor(map)]
    pub struct Attribute<'a> {
        #[n(0)]
        pub name: Cow<'a, str>,
        #[n(1)]
        pub value: Cow<'a, str>,
    }

    impl<'a> Attribute<'a> {
        pub fn new<S: Into<Cow<'a, str>>>(name: S, value: S) -> Self {
            Self {
                name: name.into(),
                value: value.into(),
            }
        }
    }

    // Trait impl

    #[async_trait::async_trait]
    impl<'a> TokenProvider<'a> for MessagingClient {
        type T = EnrollmentToken<'a>;

        async fn token(&mut self, identity: &Identity) -> ockam_core::Result<Self::T> {
            let target = "ockam_api::cloud::enroll::token";
            let label = "token";

            let route = self
                .route
                .modify()
                .append("enrollment_token_authenticator")
                .into();
            let body = RequestEnrollmentToken::new(
                identity.clone(),
                &[
                    Attribute::new("attr1", "value"),
                    Attribute::new("attr2", "value"),
                ], // TODO: define default attributes
            );
            let req = Request::post("v0/").body(body);
            self.buf = self.request(target, label, route, &req).await?;

            let mut d = Decoder::new(&self.buf);
            let res = response(target, label, &mut d)?;
            if res.status() == Some(Status::Ok) {
                d.decode().map_err(|e| e.into())
            } else {
                Err(error(target, label, &res, &mut d))
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::compat::rand::{self, Rng};
    use ockam_core::route;
    use ockam_core::{Routed, Worker};
    use ockam_identity::IdentityIdentifier;
    use ockam_node::Context;
    use ockam_transport_tcp::{TcpTransport, TCP};

    use crate::cloud::enroll::auth0::AuthorizedAuth0Token;
    use crate::cloud::enroll::enrollment_token::{
        AuthorizedEnrollmentToken, EnrollmentToken, RequestEnrollmentToken,
    };
    use crate::cloud::enroll::Token;
    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};

    use super::*;

    mod auth0 {
        use crate::cloud::enroll::auth0::*;

        use super::*;

        // TODO: add tests for the auth0 internals using mockito
        // async fn token__happy_path() {}
        // async fn token__err_if_device_token_request_fails() {}
        // async fn token__err_if_tokens_request_fails() {}

        #[ockam_macros::test]
        async fn authenticate__happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address), "cloud"];
            ctx.start_worker("cloud", EnrollHandler).await?;

            // Execute token
            let mut rng = Gen::new(32);
            let t = RandomAuthorizedAuth0Token::arbitrary(&mut rng);
            let token = AuthorizedToken::Auth0(t.0);
            let mut client = MessagingClient::new(server_route, ctx).await?;
            client.authenticate(token).await?;

            ctx.stop().await
        }

        #[derive(Debug, Clone)]
        struct RandomAuthorizedAuth0Token(AuthorizedAuth0Token<'static>);

        impl Arbitrary for RandomAuthorizedAuth0Token {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedAuth0Token(AuthorizedAuth0Token::new(
                    Identity::arbitrary(g),
                    Auth0Token {
                        token_type: TokenType::Bearer,
                        access_token: Token::arbitrary(g),
                    },
                ))
            }
        }
    }

    mod enrollment_token {
        use crate::cloud::enroll::enrollment_token::*;

        use super::*;

        #[ockam_macros::test]
        async fn token__happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address)];
            ctx.start_worker("enrollment_token_authenticator", EnrollHandler)
                .await?;

            // Execute token
            let mut rng = Gen::new(32);
            let identity = Identity::arbitrary(&mut rng);
            let mut client = MessagingClient::new(server_route, ctx).await?;
            let res = client.token(&identity).await?;
            let expected_token = EnrollmentToken::new(Token("ok".into()));
            assert_eq!(res.token, expected_token.token);

            ctx.stop().await
        }

        #[ockam_macros::test]
        async fn authenticate__happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address)];
            ctx.start_worker("enrollment_token_authenticator", EnrollHandler)
                .await?;

            // Execute token
            let mut rng = Gen::new(32);
            let t = RandomAuthorizedEnrollmentToken::arbitrary(&mut rng);
            let token = AuthorizedToken::EnrollmentToken(t.0);
            let mut client = MessagingClient::new(server_route, ctx).await?;
            client.authenticate(token).await?;

            ctx.stop().await
        }

        #[ockam_macros::test]
        #[ignore]
        async fn cloud__token(ctx: &mut Context) -> ockam_core::Result<()> {
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001")];
            let mut rng = Gen::new(32);
            let identity = Identity::arbitrary(&mut rng);
            let mut client = MessagingClient::new(server_route, ctx).await?;
            client.token(&identity).await?;
            ctx.stop().await
        }

        #[ockam_macros::test]
        #[ignore]
        async fn cloud__authenticate(ctx: &mut Context) -> ockam_core::Result<()> {
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001")];
            let mut rng = Gen::new(32);
            let t = RandomAuthorizedEnrollmentToken::arbitrary(&mut rng);
            let token = AuthorizedToken::EnrollmentToken(t.0);
            let mut client = MessagingClient::new(server_route, ctx).await?;
            client.authenticate(token).await?;
            ctx.stop().await
        }

        #[ockam_macros::test]
        #[ignore]
        async fn cloud__enroll(ctx: &mut Context) -> ockam_core::Result<()> {
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001")];
            let mut api_client = MessagingClient::new(server_route, ctx).await?;
            let identifier = random_identifier();
            let _res = api_client.enroll_enrollment_token(identifier).await?;
            ctx.stop().await
        }

        #[derive(Debug, Clone)]
        struct RandomAuthorizedEnrollmentToken(AuthorizedEnrollmentToken<'static>);

        impl Arbitrary for RandomAuthorizedEnrollmentToken {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedEnrollmentToken(AuthorizedEnrollmentToken::new(
                    Identity::arbitrary(g),
                    EnrollmentToken::new(Token::arbitrary(g)),
                ))
            }
        }
    }

    impl Arbitrary for Identity<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            Identity(String::arbitrary(g).into())
        }
    }

    impl Arbitrary for Token<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            Token(String::arbitrary(g).into())
        }
    }

    fn random_identifier() -> IdentityIdentifier {
        let id: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        IdentityIdentifier::from_key_id(id)
    }

    pub struct EnrollHandler;

    #[ockam_core::worker]
    impl Worker for EnrollHandler {
        type Message = Vec<u8>;
        type Context = Context;

        async fn handle_message(
            &mut self,
            ctx: &mut Context,
            msg: Routed<Self::Message>,
        ) -> ockam_core::Result<()> {
            let mut buf = Vec::new();
            {
                let mut dec = Decoder::new(msg.as_body());
                let req: Request = dec.decode()?;
                match (req.method(), req.path(), req.has_body()) {
                    (Some(Method::Post), "v0/", true) => {
                        if dec.decode::<RequestEnrollmentToken>().is_ok() {
                            Response::ok(req.id())
                                .body(EnrollmentToken::new(Token("ok".into())))
                                .encode(&mut buf)?;
                        } else {
                            dbg!();
                            Response::bad_request(req.id()).encode(&mut buf)?;
                        }
                    }
                    (Some(Method::Post), "v0/enroll", true) => {
                        if dec.clone().decode::<AuthorizedAuth0Token>().is_ok()
                            || dec.decode::<AuthorizedEnrollmentToken>().is_ok()
                        {
                            Response::ok(req.id()).encode(&mut buf)?;
                        } else {
                            dbg!();
                            Response::bad_request(req.id()).encode(&mut buf)?;
                        }
                    }
                    _ => {
                        dbg!();
                        Response::bad_request(req.id()).encode(&mut buf)?;
                    }
                }
            }
            ctx.send(msg.return_route(), buf).await
        }
    }
}
