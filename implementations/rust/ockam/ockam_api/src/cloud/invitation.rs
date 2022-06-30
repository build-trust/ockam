use minicbor::{Decode, Encode};

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

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
    #[b(1)] pub identity: CowStr<'a>,
    #[b(2)] pub invitee: CowStr<'a>,
    #[b(3)] pub scope: Scope,
    #[b(4)] pub space_id: CowStr<'a>,
    #[b(5)] pub project_id: Option<CowStr<'a>>,
}

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

impl<'a> CreateInvitation<'a> {
    pub fn new<S: Into<CowStr<'a>>>(
        identity: S,
        invitee: S,
        space_id: S,
        project_id: Option<S>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
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

    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Route, Routed, Worker};
    use ockam_node::Context;

    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};

    use super::*;

    #[ockam_macros::test]
    async fn accept(ctx: &mut Context) -> ockam_core::Result<()> {
        ctx.start_worker("invitations", InvitationServer::default())
            .await?;

        let mut client = MessagingClient::new(Route::new().into(), ctx).await?;
        let pubkey = "pubkey";

        // Create invitation
        let req = CreateInvitation::new("idt1", "invitee1", "space", Some("project"));
        let i = client.create_invitation(req.clone()).await?;
        assert_eq!(&i.inviter, &req.identity);
        assert_eq!(&i.invitee, &req.invitee);
        assert_eq!(&i.space_id, &req.space_id);
        assert_eq!(&i.project_id, &req.project_id);
        assert_eq!(&i.state, &State::Pending);
        assert_eq!(&i.scope, &req.scope);

        let email = i.invitee.to_string();
        let invitation_id = i.id.to_string();

        // Check default state of invitation.
        let retrieved = client.list_invitations(&email).await?;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(&retrieved[0].id.to_string(), &invitation_id);

        // Accept invitation
        client.accept_invitations(pubkey, &invitation_id).await?;
        let i1_retrieved = client.list_invitations(&email).await?;
        assert_eq!(&i1_retrieved[0].state, &State::Accepted);

        // Accepting the same invitation again should not fail.
        client.accept_invitations(pubkey, &invitation_id).await?;

        // Rejecting an accepted invitation should fail.
        let res = client.reject_invitations(pubkey, &invitation_id).await;
        assert!(res.is_err());

        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn reject(ctx: &mut Context) -> ockam_core::Result<()> {
        ctx.start_worker("invitations", InvitationServer::default())
            .await?;

        let mut client = MessagingClient::new(Route::new().into(), ctx).await?;
        let pubkey = "pubkey";

        // Create invitation.
        let req = CreateInvitation::new("idt1", "invitee1", "space", Some("project"));
        let i = client.create_invitation(req.clone()).await?;
        assert_eq!(&i.inviter, &req.identity);
        assert_eq!(&i.invitee, &req.invitee);
        assert_eq!(&i.space_id, &req.space_id);
        assert_eq!(&i.project_id, &req.project_id);
        assert_eq!(&i.state, &State::Pending);
        assert_eq!(&i.scope, &req.scope);

        let email = i.invitee.to_string();
        let invitation_id = i.id.to_string();

        // Check default state of invitation.
        let retrieved = client.list_invitations(&email).await?;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(&retrieved[0].id.to_string(), &invitation_id);

        // Reject invitation.
        client.reject_invitations(pubkey, &invitation_id).await?;
        let retrieved = client.list_invitations(&email).await?;
        assert_eq!(&retrieved[0].state, &State::Rejected);

        // Rejecting the same invitation again should not fail.
        client.reject_invitations(pubkey, &invitation_id).await?;

        // Accepting a rejected invitation should fail.
        let res = client.accept_invitations(pubkey, &invitation_id).await;
        assert!(res.is_err());

        ctx.stop().await
    }

    #[derive(Debug, Default)]
    pub struct InvitationServer {
        by_email: HashMap<String, Vec<String>>,
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
                            inviter: invitation.identity.to_string().into(),
                            invitee: invitation.invitee.to_string().into(),
                            scope: invitation.scope.clone(),
                            state: State::Pending,
                            space_id: invitation.space_id.to_string().into(),
                            project_id: invitation.project_id.map(|s| s.to_string().into()),
                        };
                        Response::ok(req.id()).body(&obj).encode(buf)?;
                        let id = obj.id.to_string();
                        self.by_id.insert(id.clone(), obj);
                        match self.by_email.get_mut(&invitation.invitee.to_string()) {
                            Some(invitations) => {
                                invitations.push(id);
                            }
                            None => {
                                self.by_email
                                    .insert(invitation.invitee.to_string(), vec![id]);
                            }
                        };
                    } else {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                }
                Some(Method::Get) => match req.path_segments::<2>().as_slice() {
                    // List invitations by email:
                    [_, email] => {
                        if let Some(invitation_ids) = self.by_email.get(*email) {
                            let invitations = invitation_ids
                                .iter()
                                .map(|id| self.by_id.get(id).unwrap())
                                .collect::<Vec<_>>();
                            Response::ok(req.id()).body(invitations).encode(buf)?
                        } else {
                            dbg!(&req);
                            Response::not_found(req.id()).encode(buf)?
                        }
                    }
                    _ => {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                },
                Some(Method::Put) => match req.path_segments::<3>().as_slice() {
                    // Accept invitation:
                    [_, id, _pubkey] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state != State::Rejected {
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
                Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                    // Reject invitation:
                    [_, id, _pubkey] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state != State::Accepted {
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
