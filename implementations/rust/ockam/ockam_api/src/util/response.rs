use crate::{Error, Request, Response, ResponseBuilder, Status};

/// Create an error response because the request path was unknown.
pub(crate) fn unknown_path<'a>(r: &'a Request) -> ResponseBuilder<Error<'a>> {
    let mut e = Error::new(r.path()).with_message("unknown path");
    if let Some(m) = r.method() {
        e = e.with_method(m)
    }
    Response::bad_request(r.id()).body(e)
}

/// Create an error response because the request method was unknown or not allowed.
pub(crate) fn invalid_method<'a>(r: &'a Request) -> ResponseBuilder<Error<'a>> {
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
