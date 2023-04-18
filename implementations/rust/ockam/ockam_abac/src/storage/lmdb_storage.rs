use crate::tokio::task::{spawn_blocking, JoinError};
use crate::{Action, Expr, PolicyStorage, Resource};
use core::str;
use lmdb::{Cursor, Transaction};
use minicbor::{Decode, Encode};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_identity::LmdbStorage;
use std::borrow::Cow;
use tracing as log;

/// Policy storage entry.
///
/// Used instead of storing plain `Expr` values to allow for additional
/// metadata, versioning, etc.
#[derive(Debug, Encode, Decode)]
#[rustfmt::skip]
struct PolicyEntry<'a> {
    #[b(0)] expr: Cow<'a, Expr>,
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
                    let e: PolicyEntry = minicbor::decode(value)?;
                    Ok(Some(e.expr.into_owned()))
                }
                Err(lmdb::Error::NotFound) => Ok(None),
                Err(e) => Err(map_lmdb_err(e)),
            }
        };
        spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set_policy(&self, r: &Resource, a: &Action, c: &Expr) -> Result<()> {
        let v = minicbor::to_vec(PolicyEntry {
            expr: Cow::Borrowed(c),
        })?;
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
                    let x: PolicyEntry = minicbor::decode(v)?;
                    xs.push((Action::new(a), x.expr.into_owned()))
                } else {
                    log::warn!(key = %ks, "malformed key in policy database")
                }
            }
            Ok(xs)
        };
        spawn_blocking(t).await.map_err(map_join_err)?
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
