use anyhow::anyhow;
#[cfg(test)]
use fake::{Dummy, Fake, Faker};
use minicbor::{Decode, Encode};

pub use enroll::*;
use ockam::{Message, TCP};
use ockam_core::{route, Route};
// pub use project::*;
// pub use space::*;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait CloudApi {
    async fn send<Params: 'static, Body: 'static, Response: 'static>(
        &mut self,
        method: RequestMethod, //TODO: replace by a proper type after CBOR support is ready
        params: Params,
        body: Body,
    ) -> ockam::Result<Option<Response>>
    where
        Params: Send,
        Body: Message + serde::Serialize + serde::de::DeserializeOwned,
        Response: serde::de::DeserializeOwned;
}

#[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub enum RequestMethod {
    Put,
}

//TODO: replace by CBOR-type after CBOR support is ready
#[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
#[cfg_attr(test, derive(Clone, PartialEq, Dummy))]
pub struct CloudApiResponse {
    id: String,
    request_id: String,
    #[cfg_attr(test, dummy(faker = "100..600"))]
    status: u16,
    // This can be a JSON structure or a String, which known in advanced but dependant on the status.
    // We can decode it as needed ad-hoc instead of enriching this struct with generics to avoid
    // cluttering the code as much as possible.
    body: Option<String>,
    from_route: Option<String>,
    to_route: Option<String>,
}

#[allow(clippy::from_over_into)]
impl<Response> Into<ockam::Result<Option<Response>>> for CloudApiResponse
where
    Response: serde::de::DeserializeOwned,
{
    fn into(self) -> ockam_core::Result<Option<Response>> {
        if self.failed() {
            let body = match self.body {
                Some(b) => b,
                None => "error has no error description".to_string(),
            };
            Err(ockam::Error::new(
                ockam::errcode::Origin::Application,
                ockam::errcode::Kind::Other,
                anyhow!(body),
            ))
        } else {
            let body = self
                .body
                .map(|b| serde_json::from_str::<Response>(&b).unwrap()); // FIXME: remove unwrap
            Ok(body)
        }
    }
}

impl CloudApiResponse {
    // Converting this struct to a Result<T>, would make it very verbose to work with all
    // possible error sources. This method is used to react to different errors,
    // trying to recover if possible.
    pub fn failed(&self) -> bool {
        self.status >= 300
    }
}

pub struct NodeCloudApi<'a> {
    cloud_node_route: Route,
    pub ctx: &'a mut ockam::Context,
}

impl<'a> NodeCloudApi<'a> {
    const DEFAULT_CLOUD_ADDR: &'static str = "cloud.ockam.io";
    const DEFAULT_CLOUD_NODE_WORKER_NAME: &'static str = "cloud"; // TODO: to be confirmed

    fn new(ctx: &'a mut ockam::Context, cloud_addr: &str, cloud_node_worker_name: &str) -> Self {
        Self {
            cloud_node_route: route![(TCP, cloud_addr), cloud_node_worker_name],
            ctx,
        }
    }
}

impl<'a> From<&'a mut ockam::Context> for NodeCloudApi<'a> {
    fn from(ctx: &'a mut ockam::Context) -> Self {
        NodeCloudApi::new(
            ctx,
            Self::DEFAULT_CLOUD_ADDR,
            Self::DEFAULT_CLOUD_NODE_WORKER_NAME,
        )
    }
}

#[async_trait::async_trait]
impl<'a> CloudApi for NodeCloudApi<'a> {
    async fn send<Params, Payload, Response: 'static>(
        &mut self,
        _method: RequestMethod,
        _params: Params,
        payload: Payload,
    ) -> ockam::Result<Option<Response>>
    where
        Params: Send,
        Payload: Message + serde::Serialize + serde::de::DeserializeOwned,
        Response: serde::de::DeserializeOwned,
    {
        // TODO: build CBOR message out of method + params + payload
        //   e.g: https://docs.google.com/document/d/1yo4GbGhzor3vsD6eRRF-EB6sgwkDTct-yLG33ST3FL0/edit#heading=h.so1yd03v36uh
        self.ctx
            .send(self.cloud_node_route.clone(), payload)
            .await?;
        self.ctx
            .receive::<CloudApiResponse>()
            .await?
            .take()
            .body()
            .into()
    }
}

pub mod enroll {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    pub struct RequestParams;

    #[derive(Message, serde::Serialize, serde::Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(map)]
    pub struct Auth0Tokens {
        #[n(0)]
        pub token_type: TokenType,
        #[n(1)]
        pub access_token: AccessToken,
    }

    #[derive(Message, serde::Serialize, serde::Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)]
        Bearer,
    }

    #[derive(Message, serde::Serialize, serde::Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(transparent)]
    pub struct AccessToken(#[n(0)] String);
}

pub mod project {
    use super::*;

    pub mod create {
        use super::*;

        #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
        #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
        pub struct RequestParams {
            pub space_name: String,
        }

        #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
        #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
        pub struct RequestBody {
            pub project_name: String,
            pub services: Vec<String>,
        }

        #[derive(serde::Serialize, serde::Deserialize, Debug)]
        #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
        pub struct ResponseBody {
            pub id: String,
            pub name: String,
            pub services: Vec<String>,
            pub access_route: Vec<u8>,
        }
    }

    // #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
    // #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    // pub struct CreateProjectPayload {
    //     pub project_name: String,
    //     // pub services: Vec<String>,
    // }
    //
    // impl From<ProjectOpts> for CreateProjectPayload {
    //     fn from(args: ProjectOpts) -> Self {
    //         Self {
    //             project_name: args.project_name,
    //         }
    //     }
    // }
    //
    // #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
    // #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    // pub struct ShowProjectRequestPayload {
    //     pub project_name: String,
    // }
    //
    // impl From<ProjectOpts> for ShowProjectRequestPayload {
    //     fn from(args: ProjectOpts) -> Self {
    //         Self {
    //             project_name: args.project_name,
    //         }
    //     }
    // }
    //
    // #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
    // #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    // pub struct ListProjectsRequestPayload;
    //
    // #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
    // #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    // pub struct DeleteProjectRequestPayload {
    //     pub project_name: String,
    // }
    //
    // impl From<ProjectOpts> for DeleteProjectRequestPayload {
    //     fn from(args: ProjectOpts) -> Self {
    //         Self {
    //             project_name: args.project_name,
    //         }
    //     }
    // }
}

// mod space {
//     use super::*;
//     use crate::cmd::cloud::space::{SpaceCommandArgs, SpaceListCommandArgs};
//
//     #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
//     #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
//     pub struct CreateSpaceRequestPayload {
//         pub project_name: String,
//         pub space_name: String,
//     }
//
//     #[derive(serde::Serialize, serde::Deserialize)]
//     #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
//     pub struct CreateSpaceResponseBody {
//         pub id: String,
//         pub project_name: String,
//         pub space_name: String,
//     }
//
//     impl From<SpaceCommandArgs> for CreateSpaceRequestPayload {
//         fn from(args: SpaceCommandArgs) -> Self {
//             Self {
//                 project_name: args.project_name,
//                 space_name: args.space_name,
//             }
//         }
//     }
//
//     #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
//     #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
//     pub struct ShowSpacePayload {
//         pub project_name: String,
//         pub space_name: String,
//     }
//
//     impl From<SpaceCommandArgs> for ShowSpacePayload {
//         fn from(args: SpaceCommandArgs) -> Self {
//             Self {
//                 project_name: args.project_name,
//                 space_name: args.space_name,
//             }
//         }
//     }
//
//     #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
//     #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
//     pub struct ListSpacesPayload {
//         pub project_name: String,
//     }
//
//     impl From<SpaceListCommandArgs> for ListSpacesPayload {
//         fn from(args: SpaceListCommandArgs) -> Self {
//             Self {
//                 project_name: args.project_name,
//             }
//         }
//     }
//
//     #[derive(Message, serde::Serialize, serde::Deserialize, Debug)]
//     #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
//     pub struct DeleteSpacePayload {
//         pub project_name: String,
//         pub space_name: String,
//     }
//
//     impl From<SpaceCommandArgs> for DeleteSpacePayload {
//         fn from(args: SpaceCommandArgs) -> Self {
//             Self {
//                 project_name: args.project_name,
//                 space_name: args.space_name,
//             }
//         }
//     }
// }

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) mod node_api {
        use std::marker::PhantomData;

        use rand::Rng;

        use ockam::TcpTransport;
        use ockam::{Routed, Worker};

        use super::*;

        pub(crate) async fn setup_node_api(
            ctx: &mut ockam::Context,
        ) -> ockam::Result<(NodeCloudApi<'_>, String)> {
            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let listener_address = transport.listen("127.0.0.1:0").await?.to_string();

            // Create Node API instance
            let worker_name: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            Ok((
                NodeCloudApi::new(ctx, &listener_address, &worker_name),
                worker_name,
            ))
        }

        pub(crate) async fn send_payload<'a, Payload>(
            node_api: &mut NodeCloudApi<'a>,
            worker_name: &str,
            payload: Payload,
        ) -> ockam::Result<()>
        where
            Payload: Message + serde::Serialize + serde::de::DeserializeOwned + Sync,
        {
            // Initiate cloud worker
            let cloud_response: CloudApiResponse = Faker.fake();
            let cloud_worker = TestCloudWorker::<Payload>::new(cloud_response.clone());
            node_api.ctx.start_worker(worker_name, cloud_worker).await?;

            let response = node_api
                .send::<_, _, ()>(RequestMethod::Put, (), payload)
                .await;
            if cloud_response.failed() {
                assert!(response.is_err());
            } else {
                assert!(response.is_ok());
            }

            node_api.ctx.stop_worker(worker_name).await?;
            tokio::time::sleep(tokio::time::Duration::from_nanos(100)).await;

            Ok(())
        }

        pub struct TestCloudWorker<T>
        where
            T: Message + serde::Serialize + serde::de::DeserializeOwned,
        {
            response: CloudApiResponse,
            _marker: PhantomData<T>,
        }

        impl<T> TestCloudWorker<T>
        where
            T: Message + serde::Serialize + serde::de::DeserializeOwned,
        {
            pub fn new(response: CloudApiResponse) -> Self {
                Self {
                    response,
                    _marker: PhantomData::default(),
                }
            }
        }

        #[ockam::worker]
        impl<T> Worker for TestCloudWorker<T>
        where
            T: Message + serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
        {
            type Message = T;
            type Context = ockam::Context;

            async fn handle_message(
                &mut self,
                ctx: &mut ockam::Context,
                msg: Routed<Self::Message>,
            ) -> ockam::Result<()> {
                ctx.send(msg.return_route(), self.response.clone()).await
            }
        }
    }
}
