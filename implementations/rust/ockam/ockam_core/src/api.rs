#![allow(missing_docs)]

use crate::compat::borrow::Cow;
use crate::compat::rand;
use crate::compat::vec::Vec;
use crate::errcode::{Kind, Origin};
use crate::Result;
use core::fmt::{self, Display, Formatter};
use minicbor::encode::{self, Encoder, Write};
use minicbor::{Decode, Decoder, Encode};
use tinyvec::ArrayVec;

#[cfg(feature = "tag")]
use crate::TypeTag;

/// A request header.
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Request<'a> {
    /// Nominal type tag.
    ///
    /// If the "tag" feature is enabled, the resulting CBOR will contain a
    /// unique numeric value that identifies this type to help catching type
    /// errors. Otherwise this tag will not be produced and is ignored during
    /// decoding if present.
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7586022>,
    /// The request identifier.
    #[n(1)] id: Id,
    /// The resource path.
    #[b(2)] path: Cow<'a, str>,
    /// The request method.
    ///
    /// It is wrapped in an `Option` to be forwards compatible, i.e. adding
    /// methods will not cause decoding errors and client code can decide
    /// how to handle unknown methods.
    #[n(3)] method: Option<Method>,
    /// Indicator if a request body is expected after this header.
    #[n(4)] has_body: bool
}

/// The response header.
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Response {
    /// Nominal type tag.
    ///
    /// If the "tag" feature is enabled, the resulting CBOR will contain a
    /// unique numeric value that identifies this type to help catching type
    /// errors. Otherwise this tag will not be produced and is ignored during
    /// decoding if present.
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9750358>,
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
    #[n(4)] has_body: bool
}

/// Create an error response because the request path was unknown.
pub fn unknown_path<'a>(r: &'a Request) -> ResponseBuilder<Error<'a>> {
    bad_request(r, "unknown path")
}

/// Create an error response because the request method was unknown or not allowed.
pub fn invalid_method<'a>(r: &'a Request) -> ResponseBuilder<Error<'a>> {
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

/// Create an error response with status forbidden and the given message.
pub fn forbidden<'a>(r: &'a Request, m: &'a str) -> ResponseBuilder<Error<'a>> {
    let mut e = Error::new(r.path()).with_message(m);
    if let Some(m) = r.method() {
        e = e.with_method(m)
    }
    Response::builder(r.id(), Status::Forbidden).body(e)
}

/// Create a generic bad request response.
pub fn bad_request<'a>(r: &'a Request, msg: &'a str) -> ResponseBuilder<Error<'a>> {
    let mut e = Error::new(r.path()).with_message(msg);
    if let Some(m) = r.method() {
        e = e.with_method(m)
    }
    Response::bad_request(r.id()).body(e)
}

/// A request/response identifier.
#[derive(Debug, Copy, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
#[cbor(transparent)]
pub struct Id(#[n(0)] u32);

/// Request methods.
#[derive(Debug, Copy, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Method {
    #[n(0)] Get,
    #[n(1)] Post,
    #[n(2)] Put,
    #[n(3)] Delete,
    #[n(4)] Patch
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
#[derive(Debug, Copy, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum Status {
    #[n(200)] Ok,
    #[n(400)] BadRequest,
    #[n(401)] Unauthorized,
    #[n(403)] Forbidden,
    #[n(404)] NotFound,
    #[n(409)] Conflict,
    #[n(405)] MethodNotAllowed,
    #[n(500)] InternalServerError,
    #[n(501)] NotImplemented
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match self {
            Status::Ok => "200 Ok",
            Status::BadRequest => "400 BadRequest",
            Status::Unauthorized => "401 Unauthorized",
            Status::Forbidden => "403 Forbidden",
            Status::NotFound => "404 NotFound",
            Status::Conflict => "409 Conflict",
            Status::MethodNotAllowed => "405 MethodNotAllowed",
            Status::InternalServerError => "500 InternalServerError",
            Status::NotImplemented => "501 NotImplemented",
        })
    }
}

impl Id {
    pub fn fresh() -> Self {
        Id(rand::random())
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

impl<'a> Request<'a> {
    pub fn new<P: Into<Cow<'a, str>>>(method: Method, path: P, has_body: bool) -> Self {
        Request {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            id: Id::fresh(),
            method: Some(method),
            path: path.into(),
            has_body,
        }
    }

    pub fn builder<P: Into<Cow<'a, str>>>(method: Method, path: P) -> RequestBuilder<'a> {
        RequestBuilder {
            header: Request::new(method, path, false),
            body: None,
        }
    }

    pub fn get<P: Into<Cow<'a, str>>>(path: P) -> RequestBuilder<'a> {
        Request::builder(Method::Get, path)
    }

    pub fn post<P: Into<Cow<'a, str>>>(path: P) -> RequestBuilder<'a> {
        Request::builder(Method::Post, path)
    }

    pub fn put<P: Into<Cow<'a, str>>>(path: P) -> RequestBuilder<'a> {
        Request::builder(Method::Put, path)
    }

    pub fn delete<P: Into<Cow<'a, str>>>(path: P) -> RequestBuilder<'a> {
        Request::builder(Method::Delete, path)
    }

    pub fn patch<P: Into<Cow<'a, str>>>(path: P) -> RequestBuilder<'a> {
        Request::builder(Method::Patch, path)
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn path(&self) -> &str {
        &*self.path
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

impl Response {
    pub fn new(re: Id, status: Status, has_body: bool) -> Self {
        Response {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            id: Id::fresh(),
            re,
            status: Some(status),
            has_body,
        }
    }

    pub fn builder(re: Id, status: Status) -> ResponseBuilder {
        ResponseBuilder {
            header: Response::new(re, status, false),
            body: None,
        }
    }

    pub fn ok(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::Ok)
    }

    pub fn bad_request(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::BadRequest)
    }

    pub fn not_found(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::NotFound)
    }

    pub fn not_implemented(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::NotImplemented)
    }

    pub fn unauthorized(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::Unauthorized)
    }

    pub fn internal_error(re: Id) -> ResponseBuilder {
        Response::builder(re, Status::InternalServerError)
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
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Error<'a> {
    /// Nominal type tag.
    ///
    /// If the "tag" feature is enabled, the resulting CBOR will contain a
    /// unique numeric value that identifies this type to help catching type
    /// errors. Otherwise this tag will not be produced and is ignored during
    /// decoding if present.
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5359172>,
    /// The resource path of this error.
    #[b(1)] path: Cow<'a, str>,
    /// The request method of this error.
    #[n(2)] method: Option<Method>,
    /// The actual error message.
    #[b(3)] message: Option<Cow<'a, str>>,
}

impl<'a> Error<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(path: S) -> Self {
        Error {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            method: None,
            path: path.into(),
            message: None,
        }
    }

    pub fn with_method(mut self, m: Method) -> Self {
        self.method = Some(m);
        self
    }

    pub fn with_message<S: Into<Cow<'a, str>>>(mut self, m: S) -> Self {
        self.message = Some(m.into());
        self
    }

    pub fn path(&self) -> &str {
        &*self.path
    }

    pub fn method(&self) -> Option<Method> {
        self.method
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
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
pub struct RequestBuilder<'a, T = ()> {
    header: Request<'a>,
    body: Option<T>,
}

impl<'a, T> RequestBuilder<'a, T> {
    pub fn id(mut self, id: Id) -> Self {
        self.header.id = id;
        self
    }

    pub fn path<P: Into<Cow<'a, str>>>(mut self, path: P) -> Self {
        self.header.path = path.into();
        self
    }

    pub fn method(mut self, m: Method) -> Self {
        self.header.method = Some(m);
        self
    }

    pub fn header(&self) -> &Request<'a> {
        &self.header
    }

    pub fn into_parts(self) -> (Request<'a>, Option<T>) {
        (self.header, self.body)
    }
}

impl<'a> RequestBuilder<'a, ()> {
    pub fn body<T: Encode<()>>(self, b: T) -> RequestBuilder<'a, T> {
        let mut b = RequestBuilder {
            header: self.header,
            body: Some(b),
        };
        b.header.has_body = true;
        b
    }
}

impl<'a, T: Encode<()>> RequestBuilder<'a, T> {
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

#[derive(Debug)]
pub struct ResponseBuilder<T = ()> {
    header: Response,
    body: Option<T>,
}

impl<T> ResponseBuilder<T> {
    pub fn id(mut self, id: Id) -> Self {
        self.header.id = id;
        self
    }

    pub fn re(mut self, re: Id) -> Self {
        self.header.re = re;
        self
    }

    pub fn status(mut self, s: Status) -> Self {
        self.header.status = Some(s);
        self
    }

    pub fn header(&self) -> &Response {
        &self.header
    }

    pub fn into_parts(self) -> (Response, Option<T>) {
        (self.header, self.body)
    }
}

impl ResponseBuilder<()> {
    pub fn body<T: Encode<()>>(self, b: T) -> ResponseBuilder<T> {
        let mut b = ResponseBuilder {
            header: self.header,
            body: Some(b),
        };
        b.header.has_body = true;
        b
    }
}

impl<T: Encode<()>> ResponseBuilder<T> {
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

#[allow(unused_variables)]
pub fn assert_request_match<'a>(schema: impl Into<Option<&'a str>>, cbor: &[u8]) {
    #[cfg(feature = "tag")]
    {
        use cddl_cat::validate_cbor_bytes;

        let mut dec = Decoder::new(cbor);
        dec.decode::<Request>().expect("header");

        if let Err(e) = validate_cbor_bytes("request", SCHEMA, &cbor[..dec.position()]) {
            tracing::error!(error = %e, "request header mismatch")
        }

        if let Some(schema) = schema.into() {
            if let Err(e) = validate_cbor_bytes(schema, SCHEMA, &cbor[dec.position()..]) {
                tracing::error!(%schema, error = %e, "request body mismatch")
            }
        }
    }
}

#[allow(unused_variables)]
pub fn assert_response_match<'a>(schema: impl Into<Option<&'a str>>, cbor: &[u8]) {
    #[cfg(feature = "tag")]
    {
        use cddl_cat::validate_cbor_bytes;

        let mut dec = Decoder::new(cbor);
        dec.decode::<Response>().expect("header");

        if let Err(e) = validate_cbor_bytes("response", SCHEMA, &cbor[..dec.position()]) {
            tracing::error!(error = %e, "response header mismatch")
        }

        if let Some(schema) = schema.into() {
            if let Err(e) = validate_cbor_bytes(schema, SCHEMA, &cbor[dec.position()..]) {
                tracing::error!(%schema, error = %e, "response body mismatch")
            }
        }
    }
}

/// Decode response header only, without processing the message body.
pub fn is_ok(label: &str, buf: &[u8]) -> Result<()> {
    let mut d = Decoder::new(buf);
    let res = response(label, &mut d)?;
    assert_response_match(None, buf);
    if res.status() == Some(Status::Ok) {
        Ok(())
    } else {
        Err(error(label, &res, &mut d))
    }
}

/// Decode response and an optional body.
pub fn decode_option<'a, 'b, T: Decode<'b, ()>>(
    label: &'a str,
    schema: impl Into<Option<&'a str>>,
    buf: &'b [u8],
) -> Result<Option<T>> {
    let mut d = Decoder::new(buf);
    let res = response(label, &mut d)?;
    match res.status() {
        Some(Status::Ok) => {
            assert_response_match(schema, buf);
            Ok(Some(d.decode()?))
        }
        Some(Status::NotFound) => Ok(None),
        _ => Err(error(label, &res, &mut d)),
    }
}

/// Decode and log response header.
pub(crate) fn response(label: &str, dec: &mut Decoder<'_>) -> Result<Response> {
    let res: Response = dec.decode()?;
    trace! {
        target:  "ockam_api",
        id     = %res.id(),
        re     = %res.re(),
        status = ?res.status(),
        body   = %res.has_body(),
        "<- {label}"
    }
    Ok(res)
}

/// Decode, log and map response error to ockam_core error.
pub(crate) fn error(label: &str, res: &Response, dec: &mut Decoder<'_>) -> crate::Error {
    if res.has_body() {
        let err = match dec.decode::<Error>() {
            Ok(e) => e,
            Err(e) => return e.into(),
        };
        warn! {
            target:  "ockam_api",
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            error  = ?err.message(),
            "<- {label}"
        }
        let msg = err.message().unwrap_or(label);
        crate::Error::new(Origin::Application, Kind::Protocol, msg)
    } else {
        warn! {
            target:  "ockam_api",
            id     = %res.id(),
            re     = %res.re(),
            status = ?res.status(),
            "<- {label}"
        }
        crate::Error::new(Origin::Application, Kind::Protocol, label)
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
