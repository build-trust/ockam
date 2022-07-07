use std::borrow::Cow;

use minicbor::{Decode, Encode};

use ockam_core::{self, async_trait};

#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Encode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct RequestEnrollment<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<5136510>,
    #[n(1)] pub token: Token<'a>,
}

impl<'a> RequestEnrollment<'a> {
    pub fn new(token: Token<'a>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            token,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Encode, Decode, Debug)]
#[cfg_attr(test, derive(PartialEq, Clone))]
#[cbor(transparent)]
pub struct Token<'a>(#[n(0)] pub Cow<'a, str>);

pub enum AuthenticateToken<'a> {
    Auth0(auth0::AuthenticateAuth0Token<'a>),
    EnrollmentToken(enrollment_token::AuthenticateEnrollmentToken<'a>),
}

pub mod auth0 {
    use super::*;

    pub const DOMAIN: &str = "dev-w5hdnpc2.us.auth0.com";
    pub const CLIENT_ID: &str = "sGyXBwQfU6fjfW1gopphdV9vCLec060b";
    pub const API_AUDIENCE: &str = "https://dev-w5hdnpc2.us.auth0.com/api/v2/";
    pub const SCOPES: &str = "profile openid email";

    #[async_trait::async_trait]
    pub trait Auth0TokenProvider<'a> {
        type T;

        async fn token(&mut self) -> ockam_core::Result<Self::T>;
    }

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
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateAuth0Token<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<1058055>,
        #[n(1)] pub token_type: TokenType,
        #[n(2)] pub access_token: Token<'a>,
    }

    impl<'a> AuthenticateAuth0Token<'a> {
        pub fn new(token: Auth0Token<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token_type: token.token_type,
                access_token: token.access_token,
            }
        }
    }

    // Auxiliary types

    #[derive(serde::Deserialize, Encode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)] Bearer,
    }
}

pub mod enrollment_token {
    use crate::auth::types::Attributes;

    use super::*;

    // Main req/res types

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct RequestEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<8560526>,
        #[b(1)] pub attributes: Attributes<'a>,
    }

    impl<'a> RequestEnrollmentToken<'a> {
        pub fn new(attributes: Attributes<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                attributes,
            }
        }
    }

    #[derive(Decode, Debug)]
    #[cfg_attr(test, derive(Encode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct EnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<8932763>,
        #[n(1)] pub token: Token<'a>,
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
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<9463780>,
        #[n(1)] pub token: Token<'a>,
    }

    impl<'a> AuthenticateEnrollmentToken<'a> {
        pub fn new(token: EnrollmentToken<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token: token.token,
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use minicbor::Decoder;
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::route;
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;
    use ockam_transport_tcp::{TcpTransport, TCP};

    use crate::cloud::enroll::auth0::AuthenticateAuth0Token;
    use crate::cloud::enroll::enrollment_token::{
        AuthenticateEnrollmentToken, EnrollmentToken, RequestEnrollmentToken,
    };
    use crate::cloud::enroll::Token;
    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};
    use ockam::identity::Identity;
    use ockam_vault::Vault;

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
            // Create a Vault to safely store secret keys for Receiver.
            let vault = Vault::create();
            // Create an Identity to represent the ockam-command client.
            let client_identity = Identity::create(ctx, &vault).await?;
            // Initiate cloud TCP listener

            // Starts a secure channel listener at "api", with a freshly created
            // identity, and a EnrollHandler worker registered at "auth0_authenticator"
            crate::util::tests::start_api_listener(
                ctx,
                &vault,
                "auth0_authenticator",
                EnrollHandler,
            )
            .await?;

            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address), "api"];

            // Execute token
            let mut rng = Gen::new(32);
            let t = RandomAuthorizedAuth0Token::arbitrary(&mut rng);
            let token = AuthenticateToken::Auth0(t.0);
            let mut client = MessagingClient::new(server_route, client_identity, ctx).await?;
            client.authenticate_token(token).await?;

            ctx.stop().await
        }

        #[derive(Debug, Clone)]
        struct RandomAuthorizedAuth0Token(AuthenticateAuth0Token<'static>);

        impl Arbitrary for RandomAuthorizedAuth0Token {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedAuth0Token(AuthenticateAuth0Token::new(Auth0Token {
                    token_type: TokenType::Bearer,
                    access_token: Token::arbitrary(g),
                }))
            }
        }
    }

    mod enrollment_token {
        use crate::auth::types::Attributes;
        use crate::cloud::enroll::enrollment_token::*;

        use super::*;

        #[ockam_macros::test]
        async fn token__happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Initiate cloud TCP listener
            // Create a Vault to safely store secret keys for Receiver.
            let vault = Vault::create();
            // Create an Identity to represent the ockam-command client.
            let client_identity = Identity::create(ctx, &vault).await?;

            // Starts a secure channel listener at "api", with a freshly created
            // identity, and a EnrollHandler worker registered at "enrollment_token_authenticator"
            crate::util::tests::start_api_listener(
                ctx,
                &vault,
                "enrollment_token_authenticator",
                EnrollHandler,
            )
            .await?;

            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address), "api"];

            // Execute token
            let mut client = MessagingClient::new(server_route, client_identity, ctx).await?;
            let res = client.generate_enrollment_token(Attributes::new()).await?;
            let expected_token = EnrollmentToken::new(Token("ok".into()));
            assert_eq!(res.token, expected_token.token);

            ctx.stop().await
        }

        #[ockam_macros::test]
        async fn authenticate__happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create a Vault to safely store secret keys for Receiver.
            let vault = Vault::create();
            // Create an Identity to represent the ockam-command client.
            let client_identity = Identity::create(ctx, &vault).await?;

            // Starts a secure channel listener at "api", with a freshly created
            // identity, and a EnrollHandler worker registered at "enrollment_token_authenticator"
            crate::util::tests::start_api_listener(
                ctx,
                &vault,
                "enrollment_token_authenticator",
                EnrollHandler,
            )
            .await?;

            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address), "api"];

            // Execute token
            let mut rng = Gen::new(32);
            let token = EnrollmentToken::new(Token::arbitrary(&mut rng));
            let mut client = MessagingClient::new(server_route, client_identity, ctx).await?;
            client.authenticate_enrollment_token(token).await?;

            ctx.stop().await
        }

        #[ockam_macros::test]
        #[ignore]
        async fn cloud__token(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create a Vault to safely store secret keys for Receiver.
            let vault = Vault::create();
            // Create an Identity to represent the ockam-command client.
            let client_identity = Identity::create(ctx, &vault).await?;
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001")];
            let mut client = MessagingClient::new(server_route, client_identity, ctx).await?;
            client.generate_enrollment_token(Attributes::new()).await?;
            ctx.stop().await
        }

        #[ockam_macros::test]
        #[ignore]
        async fn cloud__authenticate(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create a Vault to safely store secret keys for Receiver.
            let vault = Vault::create();
            // Create an Identity to represent the ockam-command client.
            let client_identity = Identity::create(ctx, &vault).await?;
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001")];
            let mut rng = Gen::new(32);
            let t = RandomAuthorizedEnrollmentToken::arbitrary(&mut rng);
            let token = AuthenticateToken::EnrollmentToken(t.0);
            let mut client = MessagingClient::new(server_route, client_identity, ctx).await?;
            client.authenticate_token(token).await?;
            ctx.stop().await
        }

        #[derive(Debug, Clone)]
        struct RandomAuthorizedEnrollmentToken(AuthenticateEnrollmentToken<'static>);

        impl Arbitrary for RandomAuthorizedEnrollmentToken {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedEnrollmentToken(AuthenticateEnrollmentToken::new(
                    EnrollmentToken::new(Token::arbitrary(g)),
                ))
            }
        }
    }

    impl Arbitrary for Token<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            Token(String::arbitrary(g).into())
        }
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
                        if dec.clone().decode::<AuthenticateAuth0Token>().is_ok()
                            || dec.decode::<AuthenticateEnrollmentToken>().is_ok()
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
