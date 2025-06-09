use ockam::transport::SchemeHostnamePort;

/// This struct stores the inlet addresses created to access remote services
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicesAddresses {
    pub(crate) http: Option<SchemeHostnamePort>,
    pub(crate) logs: Option<SchemeHostnamePort>,
    pub(crate) repl: Option<SchemeHostnamePort>,
}
