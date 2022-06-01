use super::Storage;
use lmdb::{Database, Environment, Transaction};
use ockam_core::async_trait;
use ockam_core::errcode::{Kind, Origin};
use ockam_node::tokio::task::{self, JoinError};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct Store {
    env: Arc<Environment>,
    map: Database,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Store")
    }
}

impl Store {
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self, Error> {
        let p = p.as_ref().to_path_buf();
        let t = move || {
            let env = Environment::new()
                .set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR | lmdb::EnvironmentFlags::NO_TLS)
                .set_max_dbs(1)
                .open(p.as_ref())?;
            let map = env.create_db(Some("map"), lmdb::DatabaseFlags::empty())?;
            Ok(Store {
                env: Arc::new(env),
                map,
            })
        };
        task::spawn_blocking(t).await?
    }
}

#[async_trait]
impl Storage for Store {
    type Error = Error;

    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let r = d.env.begin_ro_txn()?;
            match r.get(d.map, &k) {
                Ok(value) => Ok(Some(Vec::from(value))),
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(e.into()),
            }
        };
        task::spawn_blocking(t).await?
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<(), Self::Error> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let mut w = d.env.begin_rw_txn()?;
            w.put(d.map, &k, &val, lmdb::WriteFlags::empty())?;
            w.commit()?;
            Ok(())
        };
        task::spawn_blocking(t).await?
    }

    async fn del(&self, id: &str, key: &str) -> Result<(), Self::Error> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let mut w = d.env.begin_rw_txn()?;
            w.del(d.map, &k, None)?;
            w.commit()?;
            Ok(())
        };
        task::spawn_blocking(t).await?
    }
}

#[derive(Debug)]
pub struct Error(ErrorImpl);

#[derive(Debug)]
enum ErrorImpl {
    Lmdb(lmdb::Error),
    Task(JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Lmdb(e) => e.fmt(f),
            ErrorImpl::Task(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::Lmdb(e) => Some(e),
            ErrorImpl::Task(e) => Some(e),
        }
    }
}

impl From<lmdb::Error> for Error {
    fn from(e: lmdb::Error) -> Self {
        Error(ErrorImpl::Lmdb(e))
    }
}

impl From<JoinError> for Error {
    fn from(e: JoinError) -> Self {
        Error(ErrorImpl::Task(e))
    }
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> Self {
        ockam_core::Error::new(Origin::Node, Kind::Io, e)
    }
}
