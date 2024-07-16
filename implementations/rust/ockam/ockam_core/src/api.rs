#![allow(missing_docs)]

use core::fmt::{self, Display, Formatter};
use hashbrown::HashMap;

use minicbor::data::Type;
use minicbor::encode::{self, Encoder, Write};
use minicbor::{CborLen, Decode, Decoder, Encode};
use serde::{Serialize, Serializer};
use tinyvec::ArrayVec;

use crate::alloc::string::ToString;
use crate::compat::boxed::Box;
use crate::compat::rand;
use crate::compat::string::String;
use crate::compat::vec::Vec;
use crate::errcode::{Kind, Origin};
use crate::Result;

/// A request header.
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct RequestHeader {
    /// The request identifier.
    #[n(1)] id: Id,
    /// The resource path.
    #[n(2)] pub path: String,
    /// The request method.
    ///
    /// It is wrapped in an `Option` to be forwards compatible, i.e. adding
    /// methods will not cause decoding errors and client code can decide
    /// how to handle unknown methods.
    #[n(3)] method: Option<Method>,
    /// Indicator if a request body is expected after this header.
    #[n(4)] has_body: bool,
}

impl RequestHeader {
    pub fn new<P: Into<String>>(method: Method, path: P, has_body: bool) -> Self {
        RequestHeader {
            id: Id::fresh(),
            method: Some(method),
            path: path.into(),
            has_body,
        }
    }

    pub fn method_string(&self) -> String {
        self.method
            .map(|m| m.to_string())
            .unwrap_or("no method".to_string())
    }
}

/// The response header.
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ResponseHeader {
    /// The response identifier.
    #[n(1)] id: Id,
    /// The identifier of the request corresponding to this response.
    #[n(2)] re: Id,
    /// A status code.
    ///
    /// It is wrapped in an `Option` to be forwards compatible, i.e. adding
    /// status codes will not cause decoding errors and client code can decide
    /// how to handle unknown codes.
    #[n(3)] status: Option<Status>,
    /// Indicator if a response body is expected after this header.
    #[n(4)] has_body: bool,
}

impl ResponseHeader {
    /// Return true if the status is defined and Ok
    pub fn is_ok(&self) -> bool {
        self.status.map(|s| s == Status::Ok).unwrap_or(false)
    }

    /// If the response is not successful and the response has a body
    /// parse the response body as an error
    pub fn parse_err_msg(&self, mut dec: Decoder) -> String {
        match self.status() {
            Some(status) if self.has_body() => {
                let err = if matches!(dec.datatype(), Ok(Type::String)) {
                    dec.decode::<String>()
                        .map(|msg| format!("Message: {msg}"))
                        .unwrap_or_default()
                } else {
                    dec.decode::<Error>()
                        .map(|e| {
                            e.message()
                                .map(|msg| format!("Message: {msg}"))
                                .unwrap_or_default()
                        })
                        .unwrap_or_default()
                };
                format!(
                    "An error occurred while processing the request. Status code: {status}. {err}"
                )
            }
            Some(status) => {
                format!("An error occurred while processing the request. Status code: {status}")
            }
            None => "No status code found in response".to_string(),
        }
    }
}

/// The Reply enum separates two possible cases when interpreting a Response
///  1. there is a successfully decodable value of type T
///  2. the request failed and there is an API error (the optional status is also provided)
#[derive(Clone)]
pub enum Reply<T> {
    Successful(T),
    Failed(Error, Option<Status>),
}

impl<T: Serialize> Serialize for Reply<T> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Reply::Successful(t) => t.serialize(serializer),
            Reply::Failed(e, Some(s)) => {
                let mut map = HashMap::new();
                map.insert("error", e.to_string());
                map.insert("status", s.to_string());
                serializer.collect_map(map)
            }
            Reply::Failed(e, None) => serializer.serialize_str(&e.to_string()),
        }
    }
}

impl<T: Display> Display for Reply<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Reply::Successful(t) => f.write_str(t.to_string().as_str()),
            Reply::Failed(e, status) => {
                if let Some(m) = e.message() {
                    f.write_str(format!("Failed request: {m}").as_str())?
                } else {
                    f.write_str("Failed request")?
                };
                if let Some(status) = status {
                    f.write_str(format!("status: {status}").as_str())?
                };
                Ok(())
            }
        }
    }
}

impl<T> Reply<T> {
    /// Return the value T as a success.
    /// Any failure indicated by a non-OK status is interpreted as an error
    #[track_caller]
    pub fn success(self) -> Result<T> {
        match self {
            Reply::Successful(t) => Ok(t),
            Reply::Failed(e, _) => Err(crate::Error::new(
                Origin::Api,
                Kind::Invalid,
                e.message().unwrap_or("no message defined for this error"),
            )),
        }
    }

    #[cfg(feature = "std")]
    #[track_caller]
    pub fn miette_success(self, request_kind: &str) -> Result<T, miette::Report> {
        match self {
            Reply::Successful(t) => Ok(t),
            Reply::Failed(e, status) => {
                let message = if let Some(message) = e.message {
                    format!("Failed request to {request_kind} ({message})")
                } else {
                    format!("Failed request to {request_kind}")
                };
                let internal = internal::MietteError { message, status };
                Err(miette::Report::from(internal))
            }
        }
    }

    /// Return the value T as an option if it has been found .
    /// Any failure indicated by a non-OK or not-NotFound status is interpreted as an error
    #[track_caller]
    pub fn found(self) -> Result<Option<T>> {
        match self {
            Reply::Successful(t) => Ok(Some(t)),
            Reply::Failed(_, Some(Status::NotFound)) => Ok(None),
            Reply::Failed(e, _) => Err(crate::Error::new(
                Origin::Api,
                Kind::Invalid,
                e.message().unwrap_or("no message defined for this error"),
            )),
        }
    }
}

/// A request/response identifier.
#[derive(Debug, Default, Copy, Clone, Encode, Decode, CborLen, PartialEq, Eq, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct Id(#[n(0)] u32);

/// Request methods.
#[derive(Debug, Copy, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Method {
    #[n(0)] Get,
    #[n(1)] Post,
    #[n(2)] Put,
    #[n(3)] Delete,
    #[n(4)] Patch,
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
        })
    }
}

/// The response status codes.
#[derive(Debug, Copy, Clone, Encode, Decode, CborLen, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Status {
    #[n(200)] Ok,
    #[n(400)] BadRequest,
    #[n(401)] Unauthorized,
    #[n(403)] Forbidden,
    #[n(404)] NotFound,
    #[n(408)] Timeout,
    #[n(409)] Conflict,
    #[n(405)] MethodNotAllowed,
    #[n(500)] InternalServerError,
    #[n(501)] NotImplemented,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            Status::Ok => "200 Ok",
            Status::BadRequest => "400 BadRequest",
            Status::Unauthorized => "401 Unauthorized",
            Status::Forbidden => "403 Forbidden",
            Status::NotFound => "404 NotFound",
            Status::Timeout => "408 Timeout",
            Status::Conflict => "409 Conflict",
            Status::MethodNotAllowed => "405 MethodNotAllowed",
            Status::InternalServerError => "500 InternalServerError",
            Status::NotImplemented => "501 NotImplemented",
        })
    }
}

impl Id {
    pub fn fresh() -> Self {
        // Ensure random Ids are not equal to 0 (the default Id):
        Id(rand::random::<u32>().saturating_add(1))
    }
}

impl From<Id> for u32 {
    fn from(n: Id) -> Self {
        n.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

impl RequestHeader {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_segments<const N: usize>(&self) -> Segments<N> {
        Segments::parse(self.path())
    }

    pub fn method(&self) -> Option<Method> {
        self.method
    }

    pub fn has_body(&self) -> bool {
        self.has_body
    }
}

impl ResponseHeader {
    pub fn new(re: Id, status: Status, has_body: bool) -> Self {
        ResponseHeader {
            id: Id::fresh(),
            re,
            status: Some(status),
            has_body,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn re(&self) -> Id {
        self.re
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }

    pub fn has_body(&self) -> bool {
        self.has_body
    }
}

/// An error type used in response bodies.
#[derive(Debug, Clone, Default, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Error {
    /// The resource path of this error.
    #[n(1)] path: Option<String>,
    /// The request method of this error.
    #[n(2)] method: Option<Method>,
    /// The actual error message.
    #[n(3)] message: Option<String>,
    /// The cause of the error, if any.
    #[b(4)] cause: Option<Box<Error>>,

}

impl Error {
    #[track_caller]
    pub fn new(path: &str) -> Self {
        Error {
            method: None,
            path: Some(path.to_string()),
            message: None,
            cause: None,
        }
    }

    #[track_caller]
    pub fn new_without_path() -> Self {
        Error {
            method: None,
            path: None,
            message: None,
            cause: None,
        }
    }

    #[track_caller]
    pub fn from_failed_request(req: &RequestHeader, message: &str) -> Error {
        let mut e = Error::new(req.path()).with_message(message);
        if let Some(m) = req.method() {
            e = e.with_method(m)
        };
        e
    }

    pub fn with_method(mut self, m: Method) -> Self {
        self.method = Some(m);
        self
    }

    pub fn set_method(&mut self, m: Method) {
        self.method = Some(m);
    }

    pub fn with_message(mut self, m: impl AsRef<str>) -> Self {
        self.message = Some(m.as_ref().to_string());
        self
    }

    pub fn with_cause(mut self, e: Error) -> Self {
        self.cause = Some(Box::new(e));
        self
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn method(&self) -> Option<Method> {
        self.method
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let fields: Vec<String> = vec![
            self.message.clone().map(|m| format!("message: {m}")),
            self.path.clone().map(|p| format!("path: {p}")),
            self.method.map(|m| format!("method: {m}")),
            self.cause.clone().map(|c| c.to_string()),
        ]
        .into_iter()
        .flatten()
        .collect();
        write!(f, "{}", fields.join(", "))
    }
}

#[cfg(feature = "std")]
mod internal {
    use crate::api::Status;
    use core::fmt::{Debug, Display, Formatter};

    impl std::error::Error for MietteError {}

    pub(crate) struct MietteError {
        pub(crate) message: String,
        pub(crate) status: Option<Status>,
    }

    impl Display for MietteError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)?;
            if let Some(status) = &self.status {
                write!(f, " ({})", status)?;
            }
            Ok(())
        }
    }

    impl Debug for MietteError {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(
                f,
                "MietteError {{ message: {:?}, code: {:?} }}",
                self.message, self.status
            )
        }
    }

    #[cfg(feature = "std")]
    impl miette::Diagnostic for MietteError {
        fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
            self.status.map(|s| {
                let s: Box<dyn Display> = Box::new(s);
                s
            })
        }
    }
}

impl From<crate::Error> for Error {
    #[track_caller]
    fn from(e: crate::Error) -> Self {
        Error {
            method: None,
            path: None,
            message: Some(e.to_string()),
            cause: None,
        }
    }
}

impl From<crate::Error> for Response<Error> {
    #[track_caller]
    fn from(e: crate::Error) -> Self {
        match e.code().kind {
            Kind::NotFound => Response::not_found_no_request(&e.to_string()),
            _ => Response::internal_error_no_request(&e.to_string()),
        }
    }
}

impl From<minicbor::decode::Error> for Response<Error> {
    #[track_caller]
    fn from(e: minicbor::decode::Error) -> Self {
        Response::bad_request_no_request(&e.to_string())
    }
}

/// Path segments, i.e. '/'-separated string slices.
pub struct Segments<'a, const N: usize>(ArrayVec<[&'a str; N]>);

impl<'a, const N: usize> Segments<'a, N> {
    pub fn parse(s: &'a str) -> Self {
        if s.starts_with('/') {
            Self(s.trim_start_matches('/').splitn(N, '/').collect())
        } else {
            Self(s.splitn(N, '/').collect())
        }
    }

    pub fn as_slice(&self) -> &[&'a str] {
        &self.0[..]
    }
}

#[derive(Debug)]
pub struct Request<T = ()> {
    header: RequestHeader,
    body: Option<T>,
}

impl<T> Request<T> {
    pub fn id(mut self, id: Id) -> Self {
        self.header.id = id;
        self
    }

    pub fn path<P: Into<String>>(mut self, path: P) -> Self {
        self.header.path = path.into();
        self
    }

    pub fn method(mut self, m: Method) -> Self {
        self.header.method = Some(m);
        self
    }

    pub fn header(&self) -> &RequestHeader {
        &self.header
    }

    pub fn into_parts(self) -> (RequestHeader, Option<T>) {
        (self.header, self.body)
    }
}

impl Request {
    pub fn get<P: Into<String>>(path: P) -> Request {
        Request::build(Method::Get, path)
    }

    pub fn post<P: Into<String>>(path: P) -> Request {
        Request::build(Method::Post, path)
    }

    pub fn put<P: Into<String>>(path: P) -> Request {
        Request::build(Method::Put, path)
    }

    pub fn delete<P: Into<String>>(path: P) -> Request {
        Request::build(Method::Delete, path)
    }

    pub fn patch<P: Into<String>>(path: P) -> Request {
        Request::build(Method::Patch, path)
    }

    pub fn build<P: Into<String>>(method: Method, path: P) -> Request {
        Request {
            header: RequestHeader::new(method, path, false),
            body: None,
        }
    }
}

impl Request<()> {
    pub fn body<T: Encode<()>>(self, b: T) -> Request<T> {
        let mut b = Request {
            header: self.header,
            body: Some(b),
        };
        b.header.has_body = true;
        b
    }
}

impl<T: Encode<()>> Request<T> {
    pub fn encode<W>(&self, buf: W) -> Result<(), encode::Error<W::Error>>
    where
        W: Write,
    {
        let mut e = Encoder::new(buf);
        e.encode(&self.header)?;
        if let Some(b) = &self.body {
            e.encode(b)?;
        }
        Ok(())
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, encode::Error<<Vec<u8> as Write>::Error>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;

        Ok(buf)
    }
}

#[derive(Debug)]
pub struct Response<T = ()> {
    header: ResponseHeader,
    body: Option<T>,
}

impl<T> Response<T> {
    pub fn id(mut self, id: Id) -> Self {
        self.header.id = id;
        self
    }

    /// Setter for the Request Id on the ResponseHeader
    pub fn re(mut self, re: Id) -> Self {
        self.header.re = re;
        self
    }

    pub fn status(mut self, s: Status) -> Self {
        self.header.status = Some(s);
        self
    }

    pub fn header(&self) -> &ResponseHeader {
        &self.header
    }

    pub fn into_parts(self) -> (ResponseHeader, Option<T>) {
        (self.header, self.body)
    }
    /// Convenient wrapper to append the requests header to the response
    pub fn with_headers(self, req: &RequestHeader) -> Self {
        let id = req.id;
        self.re(id)
    }
}

impl Response<()> {
    pub fn body<T: Encode<()>>(self, b: T) -> Response<T> {
        let mut b = Response {
            header: self.header,
            body: Some(b),
        };
        b.header.has_body = true;
        b
    }
}

/// These functions create standard responses
impl Response {
    fn builder(re: Id, status: Status) -> Response {
        Response {
            header: ResponseHeader::new(re, status, false),
            body: None,
        }
    }

    pub fn error(r: &RequestHeader, msg: &str, status: Status) -> Response<Error> {
        let e = Error::from_failed_request(r, msg);
        Response::builder(r.id(), status).body(e)
    }

    pub fn ok() -> Response {
        Response::builder(Id::default(), Status::Ok)
    }

    pub fn bad_request_no_request(msg: &str) -> Response<Error> {
        let e = Error::new_without_path().with_message(msg);
        Response::builder(Id::default(), Status::BadRequest).body(e)
    }

    /// Create a generic bad request response.
    pub fn bad_request(r: &RequestHeader, msg: &str) -> Response<Error> {
        Self::error(r, msg, Status::BadRequest)
    }

    pub fn not_found(r: &RequestHeader, msg: &str) -> Response<Error> {
        Self::error(r, msg, Status::NotFound)
    }

    pub fn not_found_no_request(msg: &str) -> Response<Error> {
        let e = Error::new_without_path().with_message(msg);
        Response::builder(Id::default(), Status::NotFound).body(e)
    }

    pub fn not_implemented(re: Id) -> Response {
        Response::builder(re, Status::NotImplemented)
    }

    pub fn unauthorized(re: Id) -> Response {
        Response::builder(re, Status::Unauthorized)
    }

    pub fn forbidden_no_request(re: Id) -> Response {
        Response::builder(re, Status::Forbidden)
    }

    /// Create an error response with status forbidden and the given message.
    pub fn forbidden(r: &RequestHeader, m: &str) -> Response<Error> {
        let mut e = Error::new(r.path()).with_message(m);
        if let Some(m) = r.method() {
            e = e.with_method(m)
        }
        Response::builder(r.id(), Status::Forbidden).body(e)
    }

    pub fn internal_error_no_request(msg: &str) -> Response<Error> {
        error!(%msg);
        let e = Error::new_without_path().with_message(msg);
        Response::builder(Id::default(), Status::InternalServerError).body(e)
    }

    /// Create an internal server error response
    pub fn internal_error(r: &RequestHeader, msg: &str) -> Response<Error> {
        let mut e = Error::new(r.path()).with_message(msg);
        if let Some(m) = r.method() {
            e = e.with_method(m)
        }
        Response::builder(r.id(), Status::InternalServerError).body(e)
    }

    /// Create an error response because the request path was unknown.
    pub fn unknown_path(r: &RequestHeader) -> Response<Error> {
        Self::bad_request(r, "unknown path")
    }

    /// Create an error response because the request method was unknown or not allowed.
    pub fn invalid_method(r: &RequestHeader) -> Response<Error> {
        match r.method() {
            Some(m) => {
                let e = Error::new(r.path()).with_method(m);
                Response::builder(r.id(), Status::MethodNotAllowed).body(e)
            }
            None => {
                let e = Error::new(r.path()).with_message("unknown method");
                Response::not_implemented(r.id()).body(e)
            }
        }
    }
}

impl Response {
    /// Parse the response header and if it is ok
    /// parse and decode the response body
    pub fn parse_response_body<T>(bytes: &[u8]) -> Result<T>
    where
        T: for<'a> Decode<'a, ()>,
    {
        Self::parse_response_reply(bytes).and_then(|r| r.success())
    }

    /// Parse the response header and if it is ok
    /// parse the response body
    pub fn parse_response_reply<T>(bytes: &[u8]) -> Result<Reply<T>>
    where
        T: for<'a> Decode<'a, ()>,
    {
        let (response, mut decoder) = Self::parse_response_header(bytes)?;
        if response.is_ok() {
            // if the response is OK, try to decode the body as T
            if response.has_body() {
                match decoder.decode() {
                    Ok(t) => Ok(Reply::Successful(t)),
                    Err(e) => {
                        #[cfg(all(feature = "alloc", feature = "minicbor/half"))]
                        error!(%e, dec = %minicbor::display(bytes), hex = %hex::encode(bytes), "Failed to decode response");
                        Err(crate::Error::new(
                            Origin::Api,
                            Kind::Serialization,
                            format!("Failed to decode response body: {}", e),
                        ))
                    }
                }
                // otherwise return a decoding error
            } else {
                Err(crate::Error::new(
                    Origin::Api,
                    Kind::Serialization,
                    "expected a message body, got nothing".to_string(),
                ))
            }
            // if the status is not ok, try to read the response body as an error
        } else {
            let error = if matches!(decoder.datatype(), Ok(Type::String)) {
                decoder
                    .decode::<String>()
                    .map(|msg| Error::new_without_path().with_message(msg))
            } else {
                decoder.decode::<Error>()
            };
            match error {
                Ok(e) => Ok(Reply::Failed(e, response.status())),
                Err(e) => Err(crate::Error::new(Origin::Api, Kind::Serialization, e)),
            }
        }
    }

    /// Parse the response header and return it + the Decoder to continue parsing if necessary
    pub fn parse_response_header(bytes: &[u8]) -> Result<(ResponseHeader, Decoder)> {
        #[cfg(all(feature = "alloc", feature = "minicbor/half"))]
        trace! {
            dec = %minicbor::display(bytes),
            hex = %hex::encode(bytes),
            "Received CBOR message"
        };

        let mut dec = Decoder::new(bytes);
        let hdr = dec.decode::<ResponseHeader>()?;
        Ok((hdr, dec))
    }
}

impl<T: Encode<()>> Response<T> {
    pub fn encode<W>(&self, buf: W) -> Result<(), encode::Error<W::Error>>
    where
        W: Write,
    {
        let mut e = Encoder::new(buf);
        e.encode(&self.header)?;
        if let Some(b) = &self.body {
            e.encode(b)?;
        }
        Ok(())
    }

    pub fn to_vec(self) -> Result<Vec<u8>, encode::Error<<Vec<u8> as Write>::Error>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;

        Ok(buf)
    }
}

/// Newtype around a byte-slice that is assumed to be CBOR-encoded.
#[derive(Debug, Copy, Clone)]
pub struct Cbor<'a>(pub &'a [u8]);

impl<C> Encode<C> for Cbor<'_> {
    fn encode<W>(&self, e: &mut Encoder<W>, _: &mut C) -> Result<(), encode::Error<W::Error>>
    where
        W: Write,
    {
        // Since we assume an existing CBOR encoding, we just append the bytes as is:
        e.writer_mut()
            .write_all(self.0)
            .map_err(encode::Error::write)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use crate::cbor::schema::tests::validate_with_schema;
    use crate::cbor_encode_preallocate;

    use super::*;

    quickcheck! {
        fn request(r: RequestHeader) -> TestResult {
            validate_with_schema("request", r)
        }

        fn response(r: ResponseHeader) -> TestResult {
            validate_with_schema("response", r)
        }

        fn error(e: Error) -> TestResult {
            validate_with_schema("error", e)
        }

        fn type_check(a: RequestHeader, b: ResponseHeader, c: Error) -> TestResult {
            let cbor_a = cbor_encode_preallocate(a).unwrap();
            let cbor_b = cbor_encode_preallocate(b).unwrap();
            let cbor_c = cbor_encode_preallocate(c).unwrap();
            assert!(minicbor::decode::<ResponseHeader>(&cbor_a).is_err());
            assert!(minicbor::decode::<Error>(&cbor_a).is_err());
            assert!(minicbor::decode::<RequestHeader>(&cbor_b).is_err());
            assert!(minicbor::decode::<Error>(&cbor_b).is_err());
            assert!(minicbor::decode::<RequestHeader>(&cbor_c).is_err());
            assert!(minicbor::decode::<ResponseHeader>(&cbor_c).is_err());
            TestResult::passed()
        }
    }

    impl Arbitrary for RequestHeader {
        fn arbitrary(g: &mut Gen) -> Self {
            RequestHeader::new(
                *g.choose(METHODS).unwrap(),
                String::arbitrary(g),
                bool::arbitrary(g),
            )
        }
    }

    impl Arbitrary for ResponseHeader {
        fn arbitrary(g: &mut Gen) -> Self {
            ResponseHeader::new(Id::fresh(), *g.choose(STATUS).unwrap(), bool::arbitrary(g))
        }
    }

    impl Arbitrary for Error {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut e = Error::new(&String::arbitrary(g));
            if bool::arbitrary(g) {
                e = e.with_method(*g.choose(METHODS).unwrap())
            }
            if bool::arbitrary(g) {
                e = e.with_message(String::arbitrary(g))
            }
            e
        }
    }

    const METHODS: &[Method] = &[
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Delete,
        Method::Patch,
    ];

    const STATUS: &[Status] = &[
        Status::Ok,
        Status::BadRequest,
        Status::NotFound,
        Status::MethodNotAllowed,
        Status::InternalServerError,
        Status::NotImplemented,
    ];
}
