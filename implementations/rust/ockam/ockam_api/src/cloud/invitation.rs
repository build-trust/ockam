use minicbor::{Decode, Encode};

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Decode, Encode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Invitation<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7088378>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub inviter: CowStr<'a>,
    #[b(3)] pub invitee: CowStr<'a>,
    #[b(4)] pub scope: Scope,
    #[b(5)] pub state: State,
    #[b(6)] pub space_id: CowStr<'a>,
    #[b(7)] pub project_id: Option<CowStr<'a>>,
}

#[derive(serde::Deserialize, Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Scope {
    #[n(0)] SpaceScope,
    #[n(1)] ProjectScope,
}

#[derive(serde::Deserialize, Encode, Decode, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum State {
    #[n(0)] Pending,
    #[n(1)] Accepted,
    #[n(2)] Rejected,
}

#[derive(Decode, Encode, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInvitation<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1886440>,
    #[b(1)] pub invitee: CowStr<'a>,
    #[b(2)] pub scope: Scope,
    #[b(3)] pub space_id: CowStr<'a>,
    #[b(4)] pub project_id: Option<CowStr<'a>>,
}

impl<'a> CreateInvitation<'a> {
    pub fn new<S: Into<CowStr<'a>>>(invitee: S, space_id: S, project_id: Option<S>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            invitee: invitee.into(),
            scope: project_id
                .as_ref()
                .map_or_else(|| Scope::SpaceScope, |_| Scope::ProjectScope),
            space_id: space_id.into(),
            project_id: project_id.map(|s| s.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use minicbor::encode::Write;
    use minicbor::Decoder;
    use quickcheck::{Arbitrary, Gen};

    use ockam::identity::Identity;
    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Route, Routed, Worker};
    use ockam_node::Context;
    use ockam_vault::Vault;

    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::SCHEMA;

        use super::*;

        #[derive(Debug, Clone)]
        struct In(Invitation<'static>);

        impl Arbitrary for In {
            fn arbitrary(g: &mut Gen) -> Self {
                let project_id: CowStr = String::arbitrary(g).into();
                In(Invitation {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g).into(),
                    inviter: String::arbitrary(g).into(),
                    invitee: String::arbitrary(g).into(),
                    scope: Scope::arbitrary(g),
                    state: State::arbitrary(g),
                    space_id: String::arbitrary(g).into(),
                    project_id: g.choose(&[None, Some(project_id)]).unwrap().clone(),
                })
            }
        }

        impl Arbitrary for State {
            fn arbitrary(g: &mut Gen) -> Self {
                g.choose(&[State::Pending, State::Accepted, State::Rejected])
                    .unwrap()
                    .clone()
            }
        }

        impl Arbitrary for Scope {
            fn arbitrary(g: &mut Gen) -> Self {
                g.choose(&[Scope::SpaceScope, Scope::ProjectScope])
                    .unwrap()
                    .clone()
            }
        }

        #[derive(Debug, Clone)]
        struct CIn(CreateInvitation<'static>);

        impl Arbitrary for CIn {
            fn arbitrary(g: &mut Gen) -> Self {
                let project_id: CowStr = String::arbitrary(g).into();
                CIn(CreateInvitation {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    invitee: String::arbitrary(g).into(),
                    scope: Scope::arbitrary(g),
                    space_id: String::arbitrary(g).into(),
                    project_id: g.choose(&[None, Some(project_id)]).unwrap().clone(),
                })
            }
        }

        quickcheck! {
            fn invitation(o: In) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("invitation", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn invitations(o: Vec<In>) -> TestResult {
                let empty: Vec<Invitation> = vec![];
                let cbor = minicbor::to_vec(&empty).unwrap();
                if let Err(e) = validate_cbor_bytes("invitations", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Invitation> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(&o).unwrap();
                if let Err(e) = validate_cbor_bytes("invitations", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_invitation(o: CIn) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_invitation", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }

    #[ockam_macros::test]
    async fn accept(ctx: &mut Context) -> ockam_core::Result<()> {
        // Create a Vault to safely store secret keys for Receiver.
        let vault = Vault::create();

        // Create an Identity to represent the ockam-command client.
        let client_identity = Identity::create(ctx, &vault).await?;

        // Starts a secure channel listener at "api", with a freshly created
        // identity, and a InvitationServer worker registered at "invitations"
        crate::util::tests::start_api_listener(
            ctx,
            &vault,
            "invitations",
            InvitationServer::default(),
        )
        .await?;

        let mut client =
            MessagingClient::new(Route::new().append("api").into(), client_identity, ctx).await?;

        // Create invitation
        let req = CreateInvitation::new("invitee1", "space", Some("project"));
        let i = client.create_invitation(req.clone()).await?;
        // TODO: the inviter must be the `client_identity` created, as that's the identity invoking
        //       the API. But the simple test server here doesn't retrieve the peer identity from the
        //       secure channel metadata.
        //assert_eq!(&i.inviter, client_identity.id);
        assert_eq!(&i.invitee, &req.invitee);
        assert_eq!(&i.space_id, &req.space_id);
        assert_eq!(&i.project_id, &req.project_id);
        assert_eq!(&i.state, &State::Pending);
        assert_eq!(&i.scope, &req.scope);

        let invitation_id = i.id.to_string();

        // Check default state of invitation.
        let retrieved = client.list_invitations().await?;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(&retrieved[0].id.to_string(), &invitation_id);

        // Accept invitation
        client.accept_invitations(&invitation_id).await?;
        let i1_retrieved = client.list_invitations().await?;
        assert_eq!(&i1_retrieved[0].state, &State::Accepted);

        // Rejecting an accepted invitation should fail.
        let res = client.reject_invitations(&invitation_id).await;
        assert!(res.is_err());

        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn reject(ctx: &mut Context) -> ockam_core::Result<()> {
        // Create a Vault to safely store secret keys for Receiver.
        let vault = Vault::create();

        // Create an Identity to represent the ockam-command client.
        let client_identity = Identity::create(ctx, &vault).await?;

        // Starts a secure channel listener at "api", with a freshly created
        // identity, and a InvitationServer worker registered at "invitations"
        crate::util::tests::start_api_listener(
            ctx,
            &vault,
            "invitations",
            InvitationServer::default(),
        )
        .await?;

        let mut client =
            MessagingClient::new(Route::new().append("api").into(), client_identity, ctx).await?;

        // Create invitation.
        let req = CreateInvitation::new("invitee1", "space", Some("project"));
        let i = client.create_invitation(req.clone()).await?;
        assert_eq!(&i.invitee, &req.invitee);
        assert_eq!(&i.space_id, &req.space_id);
        assert_eq!(&i.project_id, &req.project_id);
        assert_eq!(&i.state, &State::Pending);
        assert_eq!(&i.scope, &req.scope);

        let invitation_id = i.id.to_string();

        // Check default state of invitation.
        let retrieved = client.list_invitations().await?;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(&retrieved[0].id.to_string(), &invitation_id);

        // Reject invitation.
        client.reject_invitations(&invitation_id).await?;
        let retrieved = client.list_invitations().await?;
        assert_eq!(&retrieved[0].state, &State::Rejected);

        // Accepting a rejected invitation should fail.
        let res = client.accept_invitations(&invitation_id).await;
        assert!(res.is_err());

        ctx.stop().await
    }

    #[derive(Debug, Default)]
    pub struct InvitationServer {
        by_id: HashMap<String, Invitation<'static>>,
    }

    #[ockam_core::worker]
    impl Worker for InvitationServer {
        type Message = Vec<u8>;
        type Context = Context;

        async fn handle_message(
            &mut self,
            ctx: &mut Context,
            msg: Routed<Self::Message>,
        ) -> ockam_core::Result<()> {
            let mut buf = Vec::new();
            self.on_request(msg.as_body(), &mut buf)?;
            ctx.send(msg.return_route(), buf).await
        }
    }

    impl InvitationServer {
        fn on_request<W>(&mut self, data: &[u8], buf: W) -> ockam_core::Result<()>
        where
            W: Write<Error = Infallible>,
        {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            match req.method() {
                Some(Method::Post) if req.has_body() => {
                    if let Ok(invitation) = dec.decode::<CreateInvitation>() {
                        let obj = Invitation {
                            #[cfg(feature = "tag")]
                            tag: TypeTag,
                            id: u32::arbitrary(&mut rng).to_string().into(),
                            inviter: "inviter-id".into(),
                            invitee: invitation.invitee.to_string().into(),
                            scope: invitation.scope.clone(),
                            state: State::Pending,
                            space_id: invitation.space_id.to_string().into(),
                            project_id: invitation.project_id.map(|s| s.to_string().into()),
                        };
                        Response::ok(req.id()).body(&obj).encode(buf)?;
                        let id = obj.id.to_string();
                        self.by_id.insert(id, obj);
                    } else {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                }
                Some(Method::Get) => {
                    let invitations = self.by_id.values().collect::<Vec<_>>();
                    Response::ok(req.id()).body(invitations).encode(buf)?
                }
                Some(Method::Put) => match req.path_segments::<2>().as_slice() {
                    // Accept invitation:
                    [_, id] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state == State::Pending {
                                invitation.state = State::Accepted;
                                Response::ok(req.id()).encode(buf)?;
                            } else {
                                Response::bad_request(req.id()).encode(buf)?;
                            }
                        } else {
                            dbg!(&req);
                            Response::not_found(req.id()).encode(buf)?;
                        }
                    }
                    _ => {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                },
                Some(Method::Delete) => match req.path_segments::<2>().as_slice() {
                    // Reject invitation:
                    [_, id] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state == State::Pending {
                                invitation.state = State::Rejected;
                                Response::ok(req.id()).encode(buf)?;
                            } else {
                                Response::bad_request(req.id()).encode(buf)?;
                            }
                        } else {
                            dbg!(&req);
                            Response::not_found(req.id()).encode(buf)?;
                        }
                    }
                    _ => {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                },
                _ => {
                    dbg!(&req);
                    Response::bad_request(req.id()).encode(buf)?;
                }
            }
            Ok(())
        }
    }
}
