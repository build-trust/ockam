use crate::tokio::task::{self, JoinError};
use lmdb::{Database, Environment, Transaction};
use ockam_core::async_trait;
use ockam_core::authenticated_table::AuthenticatedTable;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Lmdb AuthenticatedTable implementation
#[derive(Clone)]
pub struct LmdbTable {
    env: Arc<Environment>,
    map: Database,
}

impl fmt::Debug for LmdbTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Store")
    }
}

impl LmdbTable {
    /// Constructor
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self> {
        let p = p.as_ref().to_path_buf();
        let t = move || {
            let env = Environment::new()
                .set_flags(lmdb::EnvironmentFlags::NO_SUB_DIR | lmdb::EnvironmentFlags::NO_TLS)
                .set_max_dbs(1)
                .open(p.as_ref())
                .map_err(map_lmdb_err)?;
            let map = env
                .create_db(Some("map"), lmdb::DatabaseFlags::empty())
                .map_err(map_lmdb_err)?;
            Ok(LmdbTable {
                env: Arc::new(env),
                map,
            })
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

#[async_trait]
impl AuthenticatedTable for LmdbTable {
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
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            w.put(d.map, &k, &val, lmdb::WriteFlags::empty())
                .map_err(map_lmdb_err)?;
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn del(&self, id: &str, key: &str) -> Result<()> {
        let d = self.clone();
        let k = format!("{id}:{key}");
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            w.del(d.map, &k, None).map_err(map_lmdb_err)?;
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
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
