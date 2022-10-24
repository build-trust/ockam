use core::str;
use lmdb::{Cursor, Database, Environment, Transaction};
use ockam_abac::{Action, Expr, PolicyStorage, Resource};
use ockam_core::async_trait;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_node::tokio::task::{self, JoinError};
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tracing as log;

/// Lmdb AuthenticatedStorage implementation
#[derive(Clone)]
pub struct LmdbStorage {
    env: Arc<Environment>,
    map: Database,
}

impl fmt::Debug for LmdbStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Store")
    }
}

impl LmdbStorage {
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
            Ok(LmdbStorage {
                env: Arc::new(env),
                map,
            })
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn write(&self, k: String, v: Vec<u8>) -> Result<()> {
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

    async fn delete(&self, k: String) -> Result<()> {
        let d = self.clone();
        let t = move || {
            let mut w = d.env.begin_rw_txn().map_err(map_lmdb_err)?;
            w.del(d.map, &k, None).map_err(map_lmdb_err)?;
            w.commit().map_err(map_lmdb_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

#[async_trait]
impl AuthenticatedStorage for LmdbStorage {
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
}

#[async_trait]
impl PolicyStorage for LmdbStorage {
    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Expr>> {
        let d = self.clone();
        let k = format!("{r}:{a}");
        let t = move || {
            let r = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            match r.get(d.map, &k) {
                Ok(value) => {
                    let e: Expr = minicbor::decode(value)?;
                    Ok(Some(e))
                }
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(map_lmdb_err(e)),
            }
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set_policy(&self, r: &Resource, a: &Action, c: &Expr) -> Result<()> {
        let v = minicbor::to_vec(c)?;
        self.write(format!("{r}:{a}"), v).await
    }

    async fn del_policy(&self, r: &Resource, a: &Action) -> Result<()> {
        self.delete(format!("{r}:{a}")).await
    }

    async fn policies(&self, r: &Resource) -> Result<Vec<(Action, Expr)>> {
        let d = self.clone();
        let r = r.clone();
        let t = move || {
            let tx = d.env.begin_ro_txn().map_err(map_lmdb_err)?;
            let mut c = tx.open_ro_cursor(d.map).map_err(map_lmdb_err)?;
            let mut xs = Vec::new();
            for entry in c.iter_from(r.as_str()) {
                let (k, v) = entry.map_err(map_lmdb_err)?;
                let ks = str::from_utf8(k).map_err(from_utf8_err)?;
                if let Some((prefix, a)) = ks.split_once(':') {
                    if prefix != r.as_str() {
                        break;
                    }
                    let x = minicbor::decode(v)?;
                    xs.push((Action::new(a), x))
                } else {
                    log::warn!(key = %ks, "malformed key in policy database")
                }
            }
            Ok(xs)
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

fn from_utf8_err(err: str::Utf8Error) -> Error {
    Error::new(Origin::Other, Kind::Invalid, err)
}
