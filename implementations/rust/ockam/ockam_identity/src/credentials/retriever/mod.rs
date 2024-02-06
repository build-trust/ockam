/// This module contains three implementations for retrieving credentials:
///
///  - MemoryCredentialRetriever: that retriever is initialized with a specific credential and always returns it.
///
///  - CachedCredentialRetriever: that retriever accesses the credentials repository and makes sure that
///      the retrieved credentials are not expired
///
///  - RemoteCredentialRetriever: that retriever accesses a remote issuer.
///      It can spawn refreshers, one per subject, which will periodically refresh a credential and
///      notify some subscribers. This is used in secure channels to make sure that credentials never expire.
///
/// ```ignore
///                                      +---------------------+
///                                      | CredentialRetriever |
///                                      +---------------------+
///                                                 ^
///                                                ^^^
///                                                 |
///            +------------------------------------+-------------------------------+
///            |                                    |                               |
///   +---------------------------+    +---------------------------+   +---------------------------+
///   | MemoryCredentialRetriever |    | CachedCredentialRetriever |   | RemoteCredentialRetriever |
///   +---------------------------+    +---------------------------+   +---------------------------+
///```
mod cached_credential_retriever;
mod credential_retriever;
mod credentials_cache;
mod memory_credential_retriever;
mod remote_retriever;

pub use cached_credential_retriever::*;
pub use credential_retriever::*;
pub use credentials_cache::*;
pub use memory_credential_retriever::*;
pub use remote_retriever::*;
