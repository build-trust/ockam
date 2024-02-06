/// This module contains several structs used to retrieve and refresh credentials from an issuer
/// located on a different node:
///
///  - IssuerClient: can create a secure channel to an issuer on another node and ask for a credential
///
///  - CredentialRequest: can be executed to issue a credential for a specific pair issuer / subject
///      It is used to make sure that we don't try to concurrently retrieve the same credential.
///      The request has a run() method calling the issuer client and then caching the issued credential.
///
///  - CredentialsCache: stores credentials locally and makes sure that they are not expired when they are retrieved from storage.
///
///  - CredentialIssuer: this struct represents an issuer that we wish to access to get credentials
///      It creates credential requests when we need to get credentials for a given subject
///
///  - CredentialRefresher: periodically asks the credential issuer to get new credentials so that we never have expired
///      credentials for a given identity. It can also notify subscribers that new credentials are available.
///      This feature is used to send a CredentialRefresh message on secure channels used by the subject.
///
///  - RemoteCredentialRetriever: implementation of a CredentialRetriever accessing a remote issuer.
///      It can spawn refreshers, one per subject.
///
///```ignore
///   +--------------+
///   | IssuerClient |
///   +--------------+
///            ^
///            |
///   +-------------------+            +------------------+
///   | CredentialRequest |----------->| CredentialsCache |
///   +-------------------+            +------------------+
///            ^                                ^
///            |                                |
///   +------------------+             +---------------------+
///   | CredentialIssuer |<------------| CredentialRefresher |
///   +------------------+             +---------------------+
///             ^                               ^
///             |                               |
///             |        +----------------------+
///             |        |
///   +---------------------------+
///   | RemoteCredentialRetriever |
///   +---------------------------+
///```
///
mod credential_issuer;
mod credential_refresher;
mod credential_request;
mod credential_retriever_options;
mod issuer_client;
mod remote_credential_retriever;

pub use credential_issuer::*;
pub use credential_refresher::*;
pub use credential_request::*;
pub use credential_retriever_options::*;
pub use issuer_client::*;
pub use remote_credential_retriever::*;
