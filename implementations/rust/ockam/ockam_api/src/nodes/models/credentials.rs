//! Credential request/response types

use minicbor::{Decode, Encode};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetCredentialRequest {
    #[n(1)] overwrite: bool,
    #[n(2)] pub identity_name: Option<String>,
}

impl GetCredentialRequest {
    pub fn new(overwrite: bool, identity_name: Option<String>) -> Self {
        Self {
            overwrite,
            identity_name,
        }
    }

    pub fn is_overwrite(&self) -> bool {
        self.overwrite
    }
}

#[derive(Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PresentCredentialRequest {
    #[b(1)] pub route: String,
    #[n(2)] pub oneway: bool,
}

impl PresentCredentialRequest {
    pub fn new(route: &MultiAddr, oneway: bool) -> Self {
        Self {
            route: route.to_string(),
            oneway,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
    use crate::schema::tests::validate_with_schema;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};
    quickcheck! {
        fn get_credential_request(g: GetCredentialRequest) -> TestResult {
            validate_with_schema("get_credential_request", g)
        }

        fn present_credential_request(g: PresentCredentialRequest) -> TestResult {
            validate_with_schema("present_credential_request", g)
        }
    }

    impl Arbitrary for GetCredentialRequest {
        fn arbitrary(g: &mut Gen) -> Self {
            GetCredentialRequest {
                overwrite: bool::arbitrary(g),
                identity_name: bool::arbitrary(g).then(|| String::arbitrary(g)),
            }
        }
    }

    impl Arbitrary for PresentCredentialRequest {
        fn arbitrary(g: &mut Gen) -> Self {
            PresentCredentialRequest {
                route: String::arbitrary(g),
                oneway: bool::arbitrary(g),
            }
        }
    }
}
