use minicbor::{Decode, Encode};

use crate::CowStr;
#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Decode, Debug)]
#[cfg_attr(test, derive(Encode, Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct Space<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<7574645>,
    #[b(1)] pub id: CowStr<'a>,
    #[b(2)] pub name: CowStr<'a>,
}

#[derive(Encode, Debug)]
#[cfg_attr(test, derive(Decode, Clone))]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSpace<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] pub tag: TypeTag<3888657>,
    #[b(1)] pub name: CowStr<'a>,
}

impl<'a> CreateSpace<'a> {
    pub fn new<S: Into<CowStr<'a>>>(name: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            name: name.into(),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use core::convert::Infallible;

    use minicbor::encode::Write;
    use minicbor::{encode, Decoder};
    use quickcheck::{Arbitrary, Gen};

    use ockam::identity::Identity;
    use ockam_core::compat::collections::HashMap;
    use ockam_core::{Route, Routed, Worker};
    use ockam_node::Context;
    use ockam_vault::Vault;

    use crate::cloud::space::CreateSpace;
    use crate::cloud::MessagingClient;
    use crate::{Method, Request, Response};

    use super::*;

    mod schema {
        use cddl_cat::validate_cbor_bytes;
        use quickcheck::{quickcheck, TestResult};

        use crate::SCHEMA;

        use super::*;

        #[derive(Debug, Clone)]
        struct Sp(Space<'static>);

        impl Arbitrary for Sp {
            fn arbitrary(g: &mut Gen) -> Self {
                Sp(Space {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    id: String::arbitrary(g).into(),
                    name: String::arbitrary(g).into(),
                })
            }
        }

        #[derive(Debug, Clone)]
        struct CSp(CreateSpace<'static>);

        impl Arbitrary for CSp {
            fn arbitrary(g: &mut Gen) -> Self {
                CSp(CreateSpace {
                    #[cfg(feature = "tag")]
                    tag: Default::default(),
                    name: String::arbitrary(g).into(),
                })
            }
        }

        quickcheck! {
            fn space(o: Sp) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn spaces(o: Vec<Sp>) -> TestResult {
                let empty: Vec<Space> = vec![];
                let cbor = minicbor::to_vec(&empty).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed();

                let o: Vec<Space> = o.into_iter().map(|p| p.0).collect();
                let cbor = minicbor::to_vec(&o).unwrap();
                if let Err(e) = validate_cbor_bytes("spaces", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }

            fn create_space(o: CSp) -> TestResult {
                let cbor = minicbor::to_vec(&o.0).unwrap();
                if let Err(e) = validate_cbor_bytes("create_space", SCHEMA, &cbor) {
                    return TestResult::error(e.to_string())
                }
                TestResult::passed()
            }
        }
    }

    #[ockam_macros::test]
    async fn basic_api_usage(ctx: &mut Context) -> ockam_core::Result<()> {
        let vault = Vault::create();

        // Create an Identity to represent the ockam-command client.
        let client_identity = Identity::create(ctx, &vault).await?;

        // Starts a secure channel listener at "api", with a freshly created
        // identity, and a SpaceServer worker registered at "spaces"
        crate::util::tests::start_api_listener(ctx, &vault, "spaces", SpaceServer::default())
            .await?;

        let mut client =
            MessagingClient::new(Route::new().append("api").into(), client_identity, ctx).await?;

        let s1 = client.create_space(CreateSpace::new("s1")).await?;
        assert_eq!(&s1.name, "s1");
        let s1_id = s1.id.to_string();

        let s1_retrieved = client.get_space(&s1_id).await?;
        assert_eq!(s1_retrieved.id, s1_id);

        let s2 = client.create_space(CreateSpace::new("s2")).await?;
        assert_eq!(&s2.name, "s2");
        let s2_id = s2.id.to_string();

        let list = client.list_spaces().await?;
        assert_eq!(list.len(), 2);

        client.delete_space(&s1_id).await?;

        let list = client.list_spaces().await?;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, s2_id);

        ctx.stop().await
    }

    #[derive(Debug, Default)]
    pub struct SpaceServer(HashMap<String, Space<'static>>);

    #[ockam_core::worker]
    impl Worker for SpaceServer {
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

    impl SpaceServer {
        fn on_request<W>(&mut self, data: &[u8], buf: W) -> ockam_core::Result<()>
        where
            W: Write<Error = Infallible>,
        {
            let mut rng = Gen::new(32);
            let mut dec = Decoder::new(data);
            let req: Request = dec.decode()?;
            match req.method() {
                Some(Method::Get) => match req.path_segments::<3>().as_slice() {
                    // Get all nodes:
                    ["v0", ""] => Response::ok(req.id())
                        .body(encode::ArrayIter::new(self.0.values()))
                        .encode(buf)?,
                    // Get a single node:
                    ["v0", id] => {
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
                    if let Ok(space) = dec.decode::<CreateSpace>() {
                        let obj = Space {
                            #[cfg(feature = "tag")]
                            tag: TypeTag,
                            id: u32::arbitrary(&mut rng).to_string().into(),
                            name: space.name.to_string().into(),
                        };
                        Response::ok(req.id()).body(&obj).encode(buf)?;
                        self.0.insert(obj.id.to_string(), obj);
                    } else {
                        dbg!(&req);
                        Response::bad_request(req.id()).encode(buf)?;
                    }
                }
                Some(Method::Delete) => match req.path_segments::<3>().as_slice() {
                    [_, id] => {
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
