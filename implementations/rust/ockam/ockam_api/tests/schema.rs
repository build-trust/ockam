use cddl_cat::validate_cbor_bytes;
use ockam_api::SCHEMA;
use ockam_core::api::{Error, Id, Method, Request, Response, Status};
use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

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

#[derive(Debug, Clone)]
struct Req(Request<'static>);

#[derive(Debug, Clone)]
struct Res(Response);

#[derive(Debug, Clone)]
struct Er(Error<'static>);

impl Arbitrary for Req {
    fn arbitrary(g: &mut Gen) -> Self {
        Req(Request::new(
            *g.choose(METHODS).unwrap(),
            String::arbitrary(g),
            bool::arbitrary(g),
        ))
    }
}

impl Arbitrary for Res {
    fn arbitrary(g: &mut Gen) -> Self {
        Res(Response::new(
            Id::fresh(),
            *g.choose(STATUS).unwrap(),
            bool::arbitrary(g),
        ))
    }
}

impl Arbitrary for Er {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut e = Error::new(String::arbitrary(g));
        if bool::arbitrary(g) {
            e = e.with_method(*g.choose(METHODS).unwrap())
        }
        if bool::arbitrary(g) {
            e = e.with_message(String::arbitrary(g))
        }
        Er(e)
    }
}

quickcheck! {
    fn request_schema(a: Req) -> TestResult {
        let cbor = minicbor::to_vec(&a.0).unwrap();
        if let Err(e) = validate_cbor_bytes("request", SCHEMA, &cbor) {
            return TestResult::error(e.to_string())
        }
        TestResult::passed()
    }

    fn response_schema(a: Res) -> TestResult {
        let cbor = minicbor::to_vec(&a.0).unwrap();
        if let Err(e) = validate_cbor_bytes("response", SCHEMA, &cbor) {
            return TestResult::error(e.to_string())
        }
        TestResult::passed()
    }

    fn error_schema(a: Er) -> TestResult {
        let cbor = minicbor::to_vec(&a.0).unwrap();
        if let Err(e) = validate_cbor_bytes("error", SCHEMA, &cbor) {
            return TestResult::error(e.to_string())
        }
        TestResult::passed()
    }

    fn type_check(a: Req, b: Res, c: Er) -> TestResult {
        let cbor_a = minicbor::to_vec(&a.0).unwrap();
        let cbor_b = minicbor::to_vec(&b.0).unwrap();
        let cbor_c = minicbor::to_vec(&c.0).unwrap();
        assert!(minicbor::decode::<Response>(&cbor_a).is_err());
        assert!(minicbor::decode::<Error>(&cbor_a).is_err());
        assert!(minicbor::decode::<Request>(&cbor_b).is_err());
        assert!(minicbor::decode::<Error>(&cbor_b).is_err());
        assert!(minicbor::decode::<Request>(&cbor_c).is_err());
        assert!(minicbor::decode::<Response>(&cbor_c).is_err());
        TestResult::passed()
    }
}
