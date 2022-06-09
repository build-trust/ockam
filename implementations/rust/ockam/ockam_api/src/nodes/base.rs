// /// Request body when creating a new node.
// #[derive(Debug, Clone, Encode, Decode)]
// #[rustfmt::skip]
// #[cbor(map)]
// pub struct CreateNode<'a> {
//     #[cfg(feature = "tag")]
//     #[n(0)] tag: TypeTag<4925167>,
//     #[b(1)] name: Cow<'a, str>
// }

// /// Response body containing node information.
// #[derive(Debug, Clone, Default, Encode, Decode)]
// #[rustfmt::skip]
// #[cbor(map)]
// pub struct NodeInfo<'a> {
//     #[cfg(feature = "tag")]
//     #[n(0)] tag: TypeTag<3796735>,
//     #[b(1)] id: Cow<'a, str>,
//     #[b(2)] name: Cow<'a, str>,
//     #[b(3)] status: Cow<'a, str>,
//     #[b(4)] url: Cow<'a, str>
// }

// impl<'a> CreateNode<'a> {
//     pub fn new<S: Into<Cow<'a, str>>>(name: S) -> Self {
//         CreateNode {
//             #[cfg(feature = "tag")]
//             tag: TypeTag,
//             name: name.into(),
//         }
//     }

//     pub fn name(&self) -> &str {
//         &self.name
//     }
// }

// impl<'a> NodeInfo<'a> {
//     pub fn new() -> Self {
//         NodeInfo::default()
//     }

//     pub fn with_id<S: Into<Cow<'a, str>>>(mut self, id: S) -> Self {
//         self.id = id.into();
//         self
//     }

//     pub fn with_name<S: Into<Cow<'a, str>>>(mut self, name: S) -> Self {
//         self.name = name.into();
//         self
//     }

//     pub fn with_status<S: Into<Cow<'a, str>>>(mut self, status: S) -> Self {
//         self.status = status.into();
//         self
//     }

//     pub fn with_url<S: Into<Cow<'a, str>>>(mut self, url: S) -> Self {
//         self.url = url.into();
//         self
//     }

//     pub fn id(&self) -> &str {
//         &self.id
//     }

//     pub fn name(&self) -> &str {
//         &self.name
//     }

//     pub fn status(&self) -> &str {
//         &self.status
//     }

//     pub fn url(&self) -> &str {
//         &self.url
//     }

//     pub fn into_owned<'b>(self) -> NodeInfo<'b> {
//         NodeInfo {
//             #[cfg(feature = "tag")]
//             tag: TypeTag,
//             id: Cow::Owned(self.id.into_owned()),
//             name: Cow::Owned(self.name.into_owned()),
//             status: Cow::Owned(self.status.into_owned()),
//             url: Cow::Owned(self.url.into_owned()),
//         }
//     }
// }
