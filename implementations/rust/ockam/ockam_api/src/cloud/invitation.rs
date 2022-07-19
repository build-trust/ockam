use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Encode, Decode, Serialize, Debug)]
#[cfg_attr(test, derive(Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Invitation<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] tag: TypeTag<7088378>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub inviter: CowStr<'a>,
    #[b(3)] pub invitee: CowStr<'a>,
    #[b(4)] pub scope: Scope,
    #[b(5)] pub state: State,
    #[b(6)] pub space_id: CowStr<'a>,
    #[b(7)] pub project_id: Option<CowStr<'a>>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Scope {
    #[n(0)] SpaceScope,
    #[n(1)] ProjectScope,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum State {
    #[n(0)] Pending,
    #[n(1)] Accepted,
    #[n(2)] Rejected,
}

#[derive(Encode, Decode, Debug)]
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

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::{BareCloudRequestWrapper, CloudRequestWrapper};
    use crate::nodes::NodeMan;
    use crate::request;
    use crate::{Request, Response, Status};

    use super::*;

    const TARGET: &str = "ockam_api::cloud::invitation";

    impl NodeMan {
        pub(crate) async fn create_invitation(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<CreateInvitation> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body = req_wrapper.req;

            let label = "create_invitation";
            trace! {
                target: TARGET,
                invitee = %req_body.invitee,
                space_id = %req_body.space_id,
                project_id = %req_body.project_id.clone().unwrap_or_else(|| "None".into()),
                "creating invitation"
            };

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "invitations");

            let req_builder = Request::post("v0/").body(req_body);
            let res = match request(ctx, label, "create_invitation", route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to create invitation");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn list_invitations(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "list_invitations";
            trace!(target: TARGET, "listing invitations");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "invitations");

            let req_builder = Request::get("v0/");
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to retrieve invitations");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn accept_invitation(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "accept_invitation";
            trace!(target: TARGET, %id, "accepting invitation");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "invitations");

            let req_builder = Request::put(format!("v0/{id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to accept invitation");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }

        pub(crate) async fn reject_invitation(
            &mut self,
            ctx: &mut Context,
            req: &Request<'_>,
            dec: &mut Decoder<'_>,
            id: &str,
        ) -> Result<Vec<u8>> {
            let req_wrapper: BareCloudRequestWrapper = dec.decode()?;
            let cloud_route = req_wrapper.route()?;

            let label = "reject_invitation";
            trace!(target: TARGET, %id, "rejecting invitation");

            let sc = self.secure_channel(cloud_route).await?;
            let route = self.cloud_service_route(&sc.to_string(), "invitations");

            let req_builder = Request::delete(format!("v0/{id}"));
            let res = match request(ctx, label, None, route, req_builder).await {
                Ok(r) => Ok(r),
                Err(err) => {
                    error!(?err, "Failed to reject invitation");
                    Ok(Response::builder(req.id(), Status::InternalServerError)
                        .body(err.to_string())
                        .to_vec()?)
                }
            };
            self.delete_secure_channel(ctx, sc).await?;
            res
        }
    }
}

#[cfg(test)]
mod tests {
    use minicbor::Decoder;
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;

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

    mod node_api {
        use crate::cloud::CloudRequestWrapper;
        use crate::nodes::NodeMan;
        use crate::{route_to_multiaddr, Status};
        use ockam_core::route;

        use super::*;

        #[ockam_macros::test]
        async fn accept(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create node manager to handle requests
            let route =
                NodeMan::test_create(ctx, "invitations", InvitationServer::default()).await?;
            let cloud_route = route_to_multiaddr(&route!["cloud"]).unwrap();

            // Create invitation
            let req = CreateInvitation::new("invitee", "s1", Some("p1"));
            let mut buf = vec![];
            Request::builder(Method::Post, "v0/invitations")
                .body(CloudRequestWrapper::new(req.clone(), &cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let i = dec.decode::<Invitation>()?;
            // TODO: the inviter must be the `client_identity` created, as that's the identity invoking
            //       the API. But the simple test server here doesn't retrieve the peer identity from the
            //       secure channel metadata.
            //assert_eq!(&i.inviter, client_identity.id);
            assert_eq!(&i.invitee, &req.invitee);
            assert_eq!(&i.space_id, &req.space_id);
            assert_eq!(&i.project_id, &req.project_id);
            assert_eq!(&i.state, &State::Pending);
            assert_eq!(&i.scope, &req.scope);
            let i_id = i.id.to_string();

            // List it
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/invitations")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Invitation>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &i_id);

            // Accept invitation
            let mut buf = vec![];
            Request::builder(Method::Put, format!("v0/invitations/{i_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));

            // Check that status has changed
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/invitations")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Invitation>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &i_id);
            assert_eq!(&list[0].state, &State::Accepted);

            // Rejecting an accepted invitation should fail
            let mut buf = vec![];
            Request::builder(Method::Delete, format!("v0/invitations/{i_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::BadRequest));

            ctx.stop().await
        }

        #[ockam_macros::test]
        async fn reject(ctx: &mut Context) -> ockam_core::Result<()> {
            // Create node manager to handle requests
            let route =
                NodeMan::test_create(ctx, "invitations", InvitationServer::default()).await?;
            let cloud_route = route_to_multiaddr(&route!["cloud"]).unwrap();

            // Create invitation
            let req = CreateInvitation::new("invitee", "s1", Some("p1"));
            let mut buf = vec![];
            Request::builder(Method::Post, "v0/invitations")
                .body(CloudRequestWrapper::new(req.clone(), &cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let i = dec.decode::<Invitation>()?;
            // TODO: the inviter must be the `client_identity` created, as that's the identity invoking
            //       the API. But the simple test server here doesn't retrieve the peer identity from the
            //       secure channel metadata.
            //assert_eq!(&i.inviter, client_identity.id);
            assert_eq!(&i.invitee, &req.invitee);
            assert_eq!(&i.space_id, &req.space_id);
            assert_eq!(&i.project_id, &req.project_id);
            assert_eq!(&i.state, &State::Pending);
            assert_eq!(&i.scope, &req.scope);
            let i_id = i.id.to_string();

            // List it
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/invitations")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Invitation>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &i_id);

            // Reject invitation
            let mut buf = vec![];
            Request::builder(Method::Delete, format!("v0/invitations/{i_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));

            // Check that status has changed
            let mut buf = vec![];
            Request::builder(Method::Get, "v0/invitations")
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::Ok));
            let list = dec.decode::<Vec<Invitation>>()?;
            assert_eq!(list.len(), 1);
            assert_eq!(&list[0].id.to_string(), &i_id);
            assert_eq!(&list[0].state, &State::Rejected);

            // Accepting a rejected invitation should fail
            let mut buf = vec![];
            Request::builder(Method::Put, format!("v0/invitations/{i_id}"))
                .body(CloudRequestWrapper::bare(&cloud_route))
                .encode(&mut buf)?;
            let response: Vec<u8> = ctx.send_and_receive(route.clone(), buf).await?;
            let mut dec = Decoder::new(&response);
            let header = dec.decode::<Response>()?;
            assert_eq!(header.status, Some(Status::BadRequest));

            ctx.stop().await
        }
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
            let r = self.on_request(msg.as_body())?;
            ctx.send(msg.return_route(), r).await
        }
    }

    impl InvitationServer {
        fn on_request(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            let r = match req.method() {
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
                        let id = obj.id.to_string();
                        self.by_id.insert(id, obj.clone());
                        Response::ok(req.id()).body(&obj).to_vec()?
                    } else {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                }
                Some(Method::Get) => {
                    let invitations = self.by_id.values().collect::<Vec<_>>();
                    Response::ok(req.id()).body(invitations).to_vec()?
                }
                Some(Method::Put) => match req.path_segments::<2>().as_slice() {
                    // Accept invitation:
                    [_, id] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state == State::Pending {
                                invitation.state = State::Accepted;
                                Response::ok(req.id()).to_vec()?
                            } else {
                                Response::bad_request(req.id()).to_vec()?
                            }
                        } else {
                            error!("Invalid request: {req:#?}");
                            Response::not_found(req.id()).to_vec()?
                        }
                    }
                    _ => {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                },
                Some(Method::Delete) => match req.path_segments::<2>().as_slice() {
                    // Reject invitation:
                    [_, id] => {
                        if let Some(invitation) = self.by_id.get_mut(*id) {
                            if invitation.state == State::Pending {
                                invitation.state = State::Rejected;
                                Response::ok(req.id()).to_vec()?
                            } else {
                                Response::bad_request(req.id()).to_vec()?
                            }
                        } else {
                            error!("Invalid request: {req:#?}");
                            Response::not_found(req.id()).to_vec()?
                        }
                    }
                    _ => {
                        error!("Invalid request: {req:#?}");
                        Response::bad_request(req.id()).to_vec()?
                    }
                },
                _ => {
                    error!("Invalid request: {req:#?}");
                    Response::bad_request(req.id()).to_vec()?
                }
            };
            Ok(r)
        }
    }
}
