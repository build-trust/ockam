use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Encode};

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Decode, Debug)]
#[cfg_attr(test, derive(Encode, Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Project<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<9056532>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub name: CowStr<'a>,
    #[b(3)] pub space_name: CowStr<'a>,
    #[b(4)] pub services: Vec<CowStr<'a>>,
    #[b(5)] pub access_route: &'a ByteSlice,
}

#[derive(Encode, Debug)]
#[cfg_attr(test, derive(Decode, Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateProject<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<8669570>,
    #[b(1)] pub name: CowStr<'a>,
    #[b(2)] pub services: Vec<CowStr<'a>>,
}

impl<'a> CreateProject<'a> {
    pub fn new<S: Into<CowStr<'a>>>(name: S, services: &'a [String]) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
            services: services
                .iter()
                .map(String::as_str)
                .map(CowStr::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use minicbor::encode::Write;
    use minicbor::{encode, Decoder};
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
        struct Pr(Project<'static>);

        impl Arbitrary for Pr {
            fn arbitrary(g: &mut Gen) -> Self {
                Pr(Project {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g).into(),
                    name: String::arbitrary(g).into(),
                    space_name: String::arbitrary(g).into(),
                    services: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                    access_route: b"route"[..].into(),
                })
            }
        }

        #[derive(Debug, Clone)]
        struct CPr(CreateProject<'static>);

        impl Arbitrary for CPr {
            fn arbitrary(g: &mut Gen) -> Self {
                CPr(CreateProject {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    name: String::arbitrary(g).into(),
                    services: vec![String::arbitrary(g).into(), String::arbitrary(g).into()],
                })
            }
        }

        quickcheck! {
            fn project(o: Pr) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn projects(o: Vec<Pr>) -> TestResult {
                let empty: Vec<Project> = vec![];
                let cbor = minicbor::to_vec(&empty).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Project> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(&o).unwrap();
                if let Err(e) = validate_cbor_bytes("projects", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_project(o: CPr) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_project", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }

    #[ockam_macros::test]
    async fn basic_api_usage(ctx: &mut Context) -> ockam_core::Result<()> {
        // Create a Vault to safely store secret keys for Receiver.
        let vault = Vault::create();

        // Create an Identity to represent the ockam-command client.
        let client_identity = Identity::create(ctx, &vault).await?;

        // Starts a secure channel listener at "api", with a freshly created
        // identity, and a ProjectServer worker registered at "projects"
        crate::util::tests::start_api_listener(ctx, &vault, "projects", ProjectServer::default())
            .await?;

        let s_id = "space-id";
        let mut client =
            MessagingClient::new(Route::new().append("api").into(), client_identity, ctx).await?;

        let p1 = client
            .create_project(s_id, CreateProject::new("p1", &["service".to_string()]))
            .await?;
        assert_eq!(&p1.name, "p1");
        assert_eq!(&p1.services, &["service"]);
        let p1_id = p1.id.to_string();
        let p1_name = p1.name.to_string();

        let p1_retrieved = client.get_project(s_id, &p1_id).await?;
        assert_eq!(p1_retrieved.id, p1_id);

        let p1_retrieved = client.get_project_by_name(s_id, &p1_name).await?;
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
            W: Write<Error = Infallible>,
        {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            match req.method() {
                Some(Method::Get) => match req.path_segments::<4>().as_slice() {
                    // Get all projects:
                    [_, _] => Response::ok(req.id())
                        .body(encode::ArrayIter::new(self.0.values()))
                        .encode(buf)?,
                    // Get a single project:
                    [_, _, id] => {
                        if let Some(n) = self.0.get(*id) {
                            Response::ok(req.id()).body(n).encode(buf)?
                        } else {
                            Response::not_found(req.id()).encode(buf)?
                        }
                    }
                    // Get a single project by name:
                    [_, _, _, name] => {
                        if let Some((_, n)) = self.0.iter().find(|(_, n)| n.name == *name) {
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
                            access_route: b"route"[..].into(),
                        };
                        Response::ok(req.id()).body(&obj).encode(buf)?;
                        self.0.insert(obj.id.to_string(), obj);
                    } else {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                }
                Some(Method::Delete) => match req.path_segments::<4>().as_slice() {
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
