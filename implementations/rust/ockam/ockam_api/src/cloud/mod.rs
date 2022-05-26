use minicbor::{Decoder, Encode};
use tracing::{trace, warn};

use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Address, Route};
use ockam_node::Context;

use crate::cloud::enroll::{Authenticator, AuthenticatorClientTrait};
use crate::cloud::project::{CreateProject, Project};
use crate::cloud::space::{CreateSpace, Space};
use crate::{Error, Response};
use crate::{Request, RequestBuilder, Status};

pub mod enroll;
pub mod project;
pub mod space;

pub struct Client {
    ctx: Context,
    route: Route,
    buf: Vec<u8>,
}

impl Client {
    pub async fn new(r: Route, ctx: &Context) -> ockam_core::Result<Self> {
        let ctx = ctx.new_detached(Address::random_local()).await?;
        Ok(Client {
            ctx,
            route: r,
            buf: Vec::new(),
        })
    }

    /// Executes an enrollment process to generate a new set of access tokens.
    pub async fn enroll<Auth>(
        &mut self,
        auth: &Authenticator,
        auth_client: Auth,
    ) -> ockam_core::Result<()>
    where
        Auth: AuthenticatorClientTrait,
    {
        let target = "ockam_api::cloud::enroll";
        let label = "enroll";
        trace!(target = %target, auth = %auth, "generating tokens");

        let req = Request::post("/authenticators/auth0/authenticate");
        let req = match auth {
            Authenticator::Auth0 => {
                let tokens = auth_client.auth0().await?;
                tracing::info!("tokens received {tokens:?}");
                req.body(tokens)
            }
            Authenticator::EnrollmentToken => unimplemented!(),
        };
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::post("/v0").body(body);
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::get("/v0");
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::get(format!("/v0/{space_id}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::post(format!("/v0/name/{space_name}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::delete(format!("/v0/{space_id}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::post(format!("/v0/{space_id}")).body(body);
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::get(format!("/v0/{space_id}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::get(format!("/v0/{space_id}/{project_id}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::post(format!("/v0/{space_id}/name/{project_name}"));
        self.buf = self.request(target, label, &req).await?;
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

        let req = Request::delete(format!("/v0/{space_id}/{project_id}"));
        self.buf = self.request(target, label, &req).await?;
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
        req: &RequestBuilder<'_, T>,
    ) -> ockam_core::Result<Vec<u8>>
    where
        T: Encode<()>,
    {
        let mut buf = Vec::new();
        req.encode(&mut buf)?;
        trace!(target = %target, label = %label, id = %req.header().id(), "-> req");
        let vec: Vec<u8> = self.ctx.send_and_receive(self.route.clone(), buf).await?;
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

mod tests {
    use ockam_core::{Routed, Worker};
    use ockam_node::Context;

    use super::*;
    use crate::{Request, Response};

    pub struct TestCloudWorker;

    #[ockam_core::worker]
    impl Worker for TestCloudWorker {
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
                Response::ok(req.id()).encode(&mut buf)?;
            }
            ctx.send(msg.return_route(), buf).await
        }
    }
}
