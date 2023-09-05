use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::tokio::task::{self, JoinError};

use crate::storage::Storage;

use core::str;
use lmdb::{Cursor, Database, Environment, Transaction};
use std::fmt;
use std::path::Path;
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;

/// Storage using the LMDB database
#[derive(Clone)]
pub struct LmdbStorage {
    /// lmdb da
    pub env: Arc<Environment>,
    /// lmdb database file
    pub map: Database,
}

impl fmt::Debug for LmdbStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Store")
    }
}

impl LmdbStorage {
    /// Constructor
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self> {
        // creating a new database might be failing a few times
        // if the files are currently being held by another pod which is shutting down.
        // In that case we retry a few times, between 1 and 10 seconds.
        let retry_strategy = FixedInterval::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(10); // limit to 10 retries

        let path: &Path = p.as_ref();
        Retry::spawn(retry_strategy, || async { Self::make(path).await }).await
    }

    async fn make(p: &Path) -> Result<Self> {
        debug!("create the LMDB database");
        std::fs::create_dir_all(p.parent().unwrap())
            .map_err(|e| Error::new(Origin::Node, Kind::Io, e))?;
        let p = p.to_path_buf();
        let env = Environment::new()
            .set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR | lmdb::EnvironmentFlags::NO_TLS)
            .set_max_dbs(1)
            .open(p.as_ref())
            .map_err(map_lmdb_err)?;
        let map = env
            .create_db(Some("map"), lmdb::DatabaseFlags::empty())
            .map_err(map_lmdb_err)?;
        Ok(LmdbStorage {
            env: Arc::new(env),
            map,
        })
    }

    /// Write a new binary value for a given key in the database
    pub async fn write(&self, k: String, v: Vec<u8>) -> Result<()> {
        let d = self.clone();
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            w.put(d.map, &k, &v, lmdb::WriteFlags::empty())
                .map_err(map_lmdb_err)?;
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    /// Delete a database entry
    pub async fn delete(&self, k: String) -> Result<()> {
        let d = self.clone();
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            match w.del(d.map, &k, None) {
                Ok(()) | Err(lmdb::Error::NotFound) => {}
                Err(e) => return Err(map_lmdb_err(e)),
            }
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

#[async_trait]
impl Storage for LmdbStorage {
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let r = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            match r.get(d.map, &k) {
                Ok(value) => Ok(Some(Vec::from(value))),
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(map_lmdb_err(e)),
            }
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()> {
        self.write(format!("{id}:{key}"), val).await
    }

    async fn del(&self, id: &str, key: &str) -> Result<()> {
        self.delete(format!("{id}:{key}")).await
    }

    async fn keys(&self, namespace: &str) -> Result<Vec<String>> {
        let d = self.clone();
        let suffix = format!(":{}", namespace);
        let t = move || {
            let r = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            let mut cursor = r.open_ro_cursor(d.map).map_err(map_lmdb_err)?;
            Ok(cursor
                .iter()
                .filter_map(|r| {
                    let (k, _) = r.unwrap();
                    let key = str::from_utf8(k).unwrap();
                    key.rsplit_once(&suffix).map(|(k, _)| k.to_string())
                })
                .collect())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_lmdb_err(err: lmdb::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}
