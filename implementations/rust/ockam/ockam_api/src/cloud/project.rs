use std::borrow::Cow;

use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Decode, Debug)]
#[cfg_attr(test, derive(Encode))]
#[cbor(map)]
pub struct Project<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    pub tag: TypeTag<2764235>,
    #[b(1)]
    pub id: Cow<'a, str>, // TODO: str or Vec<u8>?
    #[b(2)]
    pub name: Cow<'a, str>,
    #[b(3)]
    pub space_name: Cow<'a, str>,
    #[b(4)]
    pub services: Vec<Cow<'a, str>>,
    #[b(5)]
    pub access_route: Vec<u8>,
}

#[derive(Encode)]
#[cfg_attr(test, derive(Decode))]
#[cbor(map)]
pub struct CreateProject<'a> {
    #[cfg(feature = "tag")]
    #[n(0)]
    pub tag: TypeTag<6593388>,
    #[b(1)]
    pub name: Cow<'a, str>,
    #[b(2)]
    pub services: Vec<Cow<'a, str>>,
}

impl<'a> CreateProject<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(name: S, services: &'a [String]) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
            services: services
                .iter()
                .map(String::as_str)
                .map(Cow::Borrowed)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use minicbor::encode::Write;
    use minicbor::{encode, Decoder};
    use quickcheck::{Arbitrary, Gen};

    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Route, Routed, Worker};
    use ockam_node::Context;

    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};

    use super::*;

    #[ockam_macros::test]
    async fn basic_api_usage(ctx: &mut Context) -> ockam_core::Result<()> {
        ctx.start_worker("projects", ProjectServer::default())
            .await?;

        let s_id = "space-id";
        let mut client = MessagingClient::new(Route::new().into(), ctx).await?;

        let p1 = client
            .create_project(s_id, CreateProject::new("p1", &["service".to_string()]))
            .await?;
        assert_eq!(&p1.name, "p1");
        assert_eq!(&p1.services, &["service"]);
        let p1_id = p1.id.to_string();

        let p1_retrieved = client.get_project(s_id, &p1_id).await?;
        assert_eq!(p1_retrieved.id, p1_id);

        let p2 = client
            .create_project(s_id, CreateProject::new("p2", &["service".to_string()]))
            .await?;
        assert_eq!(&p2.name, "p2");
        let p2_id = p2.id.to_string();

        let list = client.list_projects(s_id).await?;
        assert_eq!(list.len(), 2);

        client.delete_project(s_id, &p1_id).await?;

        let list = client.list_projects(s_id).await?;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, p2_id);

        ctx.stop().await
    }

    #[derive(Debug, Default)]
    pub struct ProjectServer(HashMap<String, Project<'static>>);

    #[ockam_core::worker]
    impl Worker for ProjectServer {
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

    impl ProjectServer {
        fn on_request<W>(&mut self, data: &[u8], buf: W) -> ockam_core::Result<()>
        where
            W: Write<Error = io::Error>,
        {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            match req.method() {
                Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                    // Get all nodes:
                    [_, _] => Response::ok(req.id())
                        .body(encode::ArrayIter::new(self.0.values()))
                        .encode(buf)?,
                    // Get a single node:
                    [_, _, id] => {
                        if let Some(n) = self.0.get(*id) {
                            Response::ok(req.id()).body(n).encode(buf)?
                        } else {
                            Response::not_found(req.id()).encode(buf)?
                        }
                    }
                    _ => {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                },
                Some(Method::Post) if req.has_body() => {
                    if let Ok(project) = dec.decode::<CreateProject>() {
                        let obj = Project {
                            #[cfg(feature = "tag")]
                            tag: TypeTag,
                            id: u32::arbitrary(&mut rng).to_string().into(),
                            name: project.name.to_string().into(),
                            space_name: String::arbitrary(&mut rng).into(),
                            services: project
                                .services
                                .iter()
                                .map(|x| x.to_string().into())
                                .collect(),
                            access_route: vec![],
                        };
                        Response::ok(req.id()).body(&obj).encode(buf)?;
                        self.0.insert(obj.id.to_string(), obj);
                    } else {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                }
                Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                    [_, _, id] => {
                        if self.0.remove(*id).is_some() {
                            Response::ok(req.id()).encode(buf)?
                        } else {
                            Response::not_found(req.id()).encode(buf)?
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
