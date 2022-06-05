use minicbor::{Decoder, Encode};
use tracing::{trace, warn};

use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Route};
use ockam_identity::IdentityIdentifier;
use ockam_node::Context;

use crate::cloud::enroll::auth0::{Auth0Token, AuthorizedAuth0Token};
use crate::cloud::enroll::enrollment_token::AuthorizedEnrollmentToken;
use crate::cloud::enroll::{AuthorizedToken, Identity, TokenAuthenticatorService, TokenProvider};
use crate::cloud::invitation::{CreateInvitation, Invitation};
use crate::cloud::project::{CreateProject, Project};
use crate::cloud::space::{CreateSpace, Space};
use crate::{Error, Response};
use crate::{Request, RequestBuilder, Status};

pub mod enroll;
pub mod invitation;
pub mod project;
pub mod space;

pub struct MessagingClient {
    ctx: Context,
    /// The target node address, without the worker name.
    route: Route,
    buf: Vec<u8>,
}

impl MessagingClient {
    pub async fn new(route: Route, ctx: &Context) -> ockam_core::Result<Self> {
        let ctx = ctx.new_detached(Address::random_local()).await?;
        Ok(MessagingClient {
            ctx,
            route,
            buf: Vec::new(),
        })
    }

    /// Executes an enrollment process to generate a new set of access tokens using the auth0 flow.
    pub async fn enroll_auth0<'a, S>(
        &mut self,
        identifier: IdentityIdentifier,
        mut auth0_service: S,
    ) -> ockam_core::Result<()>
    where
        S: TokenProvider<'a, T = Auth0Token<'a>>,
    {
        let target = "ockam_api::cloud::enroll_auth0";
        trace!(target = %target, "generating tokens");

        let identity = Identity::from(identifier.to_string());
        let token = {
            let token = auth0_service.token(&identity).await?;
            AuthorizedToken::Auth0(AuthorizedAuth0Token::new(identity, token))
        };
        self.authenticate(token).await?;
        Ok(())
    }

    /// Executes an enrollment process to generate a new set of access tokens using the enrollment token flow.
    pub async fn enroll_enrollment_token(
        &mut self,
        identifier: IdentityIdentifier,
    ) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::enroll_enrollment_token";
        trace!(target = %target, "generating tokens");

        let identity = Identity::from(identifier.to_string());
        let token = {
            let token = self.token(&identity).await?;
            AuthorizedToken::EnrollmentToken(AuthorizedEnrollmentToken::new(identity, token))
        };
        self.authenticate(token).await?;
        Ok(())
    }

    pub async fn create_invitation(
        &mut self,
        body: CreateInvitation<'_>,
    ) -> ockam_core::Result<Invitation<'_>> {
        let target = "ockam_api::cloud::create_invitation";
        let label = "create_invitation";
        trace!(target = %target, space = %body.space_id, "creating invitation");

        let route = self.route.modify().append("invitations").into();
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

    pub async fn list_invitations(
        &mut self,
        email: &str,
    ) -> ockam_core::Result<Vec<Invitation<'_>>> {
        let target = "ockam_api::cloud::list_invitations";
        let label = "list_invitations";
        trace!(target = %target, "listing invitations");

        let route = self.route.modify().append("invitations").into();
        let req = Request::get(format!("v0/{}", email));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn accept_invitations(
        &mut self,
        email: &str,
        invitation_id: &str,
    ) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::accept_invitations";
        let label = "accept_invitation";
        trace!(target = %target, "accept invitation");

        let route = self.route.modify().append("invitations").into();
        let req = Request::put(format!("v0/{}/{}", invitation_id, email));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn reject_invitations(
        &mut self,
        email: &str,
        invitation_id: &str,
    ) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::reject_invitations";
        let label = "reject_invitation";
        trace!(target = %target, "reject invitation");

        let route = self.route.modify().append("invitations").into();
        let req = Request::delete(format!("v0/{}/{}", invitation_id, email));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn create_space(&mut self, body: CreateSpace<'_>) -> ockam_core::Result<Space<'_>> {
        let target = "ockam_api::cloud::create_space";
        let label = "create_space";
        trace!(target = %target, space = %body.name, "creating space");

        let route = self.route.modify().append("spaces").into();
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

    pub async fn list_spaces(&mut self) -> ockam_core::Result<Vec<Space<'_>>> {
        let target = "ockam_api::cloud::list_spaces";
        let label = "list_spaces";
        trace!(target = %target, "listing spaces");

        let route = self.route.modify().append("spaces").into();
        let req = Request::get("v0/");
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn get_space(&mut self, space_id: &str) -> ockam_core::Result<Space<'_>> {
        let target = "ockam_api::cloud::get_space";
        let label = "get_space";
        trace!(target = %target, space = %space_id, space = %space_id, "getting space");

        let route = self.route.modify().append("spaces").into();
        let req = Request::get(format!("v0/{space_id}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn get_space_by_name(&mut self, space_name: &str) -> ockam_core::Result<Space<'_>> {
        let target = "ockam_api::cloud::get_space_by_name";
        let label = "get_space_by_name";
        trace!(target = %target, space = %space_name, "getting space");

        let route = self.route.modify().append("spaces").into();
        let req = Request::get(format!("v0/name/{space_name}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn delete_space(&mut self, space_id: &str) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::delete_space";
        let label = "delete_space";
        trace!(target = %target, space = %space_id, "deleting space");

        let route = self.route.modify().append("spaces").into();
        let req = Request::delete(format!("v0/{space_id}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn create_project(
        &mut self,
        space_id: &str,
        body: CreateProject<'_>,
    ) -> ockam_core::Result<Project<'_>> {
        let target = "ockam_api::cloud::create_project";
        let label = "create_project";
        trace!(target = %target, space = %space_id, project = %body.name, "creating project");

        let route = self.route.modify().append("projects").into();
        let req = Request::post(format!("v0/{space_id}")).body(body);
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn list_projects(&mut self, space_id: &str) -> ockam_core::Result<Vec<Project<'_>>> {
        let target = "ockam_api::cloud::list_projects";
        let label = "list_projects";
        trace!(target = %target, space = %space_id, "listing projects");

        let route = self.route.modify().append("projects").into();
        let req = Request::get(format!("v0/{space_id}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn get_project(
        &mut self,
        space_id: &str,
        project_id: &str,
    ) -> ockam_core::Result<Project<'_>> {
        let target = "ockam_api::cloud::get_project";
        let label = "get_project";
        trace!(target = %target, space = %space_id, project = %project_id, "getting project");

        let route = self.route.modify().append("projects").into();
        let req = Request::get(format!("v0/{space_id}/{project_id}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn get_project_by_name(
        &mut self,
        space_id: &str,
        project_name: &str,
    ) -> ockam_core::Result<Project<'_>> {
        let target = "ockam_api::cloud::get_project_by_name";
        let label = "get_project_by_name";
        trace!(target = %target, space = %space_id, project = %project_name, "getting project");

        let route = self.route.modify().append("projects").into();
        let req = Request::get(format!("v0/{space_id}/name/{project_name}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            d.decode().map_err(|e| e.into())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    pub async fn delete_project(
        &mut self,
        space_id: &str,
        project_id: &str,
    ) -> ockam_core::Result<()> {
        let target = "ockam_api::cloud::delete_project";
        let label = "delete_project";
        trace!(target = %target, space = %space_id, project = %project_id, "deleting project");

        let route = self.route.modify().append("projects").into();
        let req = Request::delete(format!("v0/{space_id}/{project_id}"));
        self.buf = self.request(target, label, route, &req).await?;
        let mut d = Decoder::new(&self.buf);
        let res = response(target, label, &mut d)?;
        if res.status() == Some(Status::Ok) {
            Ok(())
        } else {
            Err(error(target, label, &res, &mut d))
        }
    }

    /// Encode request header and body (if any), send the package to the server and returns its response.
    async fn request<T>(
        &mut self,
        target: &str,
        label: &str,
        route: Route,
        req: &RequestBuilder<'_, T>,
    ) -> ockam_core::Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        trace!(target = %target, label = %label, id = %req.header().id(), route = %route, "-> req");
        let vec: Vec<u8> = self.ctx.send_and_receive(route, buf).await?;
        Ok(vec)
    }
}

/// Decode and log response header.
pub(crate) fn response(
    target: &str,
    label: &str,
    dec: &mut Decoder<'_>,
) -> ockam_core::Result<Response> {
    let res: Response = dec.decode()?;
    trace! {
        target = target,
        label  = %label,
        id     = %res.id(),
        re     = %res.re(),
        status = ?res.status(),
        body   = %res.has_body(),
        "<- res"
    }
    Ok(res)
}

/// Decode, log and mape response error to ockam_core error.
pub(crate) fn error(
    target: &str,
    label: &str,
    res: &Response,
    dec: &mut Decoder<'_>,
) -> ockam_core::Error {
    if res.has_body() {
        let err = match dec.decode::<Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target = target,
            label  = %label,
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            error  = ?err.message(),
            "<- err"
        }
        let msg = err.message().unwrap_or(label);
        ockam_core::Error::new(Origin::Application, Kind::Protocol, msg)
    } else {
        ockam_core::Error::new(Origin::Application, Kind::Protocol, label)
    }
}
