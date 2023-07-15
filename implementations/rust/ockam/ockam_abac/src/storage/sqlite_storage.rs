use crate::tokio::task::{spawn_blocking, JoinError};
use crate::{Action, Expr, PolicyStorage, Resource};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_identity::SqliteStorage;
use rusqlite::{params, ToSql};
use std::borrow::Cow;

use super::PolicyEntry;

impl ToSql for Resource {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl ToSql for Action {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

#[async_trait]
impl PolicyStorage for SqliteStorage {
    async fn get_policy(&self, r: &Resource, a: &Action) -> Result<Option<Expr>> {
        let conn = self.conn();
        let r = r.clone();
        let a = a.clone();
        let t = move || {
            let conn = conn.lock().unwrap();
            let result = conn
                .query_row::<Option<Expr>, _, _>(
                    "SELECT value FROM policy WHERE resource = ?1 AND action = ?2;",
                    params![r, a],
                    |row| {
                        row.get::<_, Vec<u8>>(0).map(|value| {
                            let e: PolicyEntry = minicbor::decode(&value).unwrap();
                            Some(e.expr.into_owned())
                        })
                    },
                )
                .map_err(map_sqlite_err);
            result
        };
        spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set_policy(&self, r: &Resource, a: &Action, c: &Expr) -> Result<()> {
        let conn = self.conn();
        let r = r.clone();
        let a = a.clone();
        let v = minicbor::to_vec(PolicyEntry {
            expr: Cow::Borrowed(c),
        })?;
        let t = move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO policy (resource, action, value) VALUES (?1, ?2, ?3)",
                params![r, a, v],
            )
            .map_err(map_sqlite_err)?;
            Ok(())
        };
        spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn del_policy(&self, r: &Resource, a: &Action) -> Result<()> {
        let conn = self.conn();
        let r = r.clone();
        let a = a.clone();
        let t = move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                "DELETE FROM policy WHERE resource = ?1 AND action = ?2;",
                params![r, a],
            )
            .map_err(map_sqlite_err)?;
            Ok(())
        };
        spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn policies(&self, r: &Resource) -> Result<Vec<(Action, Expr)>> {
        let conn = self.conn();
        let r = r.clone();
        let t = move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT action, value FROM policy WHERE resource = ?1;")
                .map_err(map_sqlite_err)?;
            let result = stmt
                .query_map::<(Action, Vec<u8>), _, _>(params![r], |row| {
                    let action: Action = Action::from(row.get::<_, String>(0)?);
                    let value: Vec<u8> = row.get(1)?;
                    Ok((action, value))
                })
                .map_err(map_sqlite_err)?
                .map(
                    |value: core::result::Result<(Action, Vec<u8>), rusqlite::Error>| {
                        value.map_err(map_sqlite_err)
                    },
                )
                .collect::<Result<Vec<(Action, Vec<u8>)>, Error>>()?;
            let decoded_result = result
                .iter()
                .map(|(action, value)| {
                    let e: PolicyEntry = minicbor::decode(value).map_err(map_decode_err)?;
                    Ok((action.to_owned(), e.expr.into_owned()))
                })
                .collect();
            decoded_result
        };
        spawn_blocking(t).await.map_err(map_join_err)?
    }
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_sqlite_err(err: rusqlite::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_decode_err(err: minicbor::decode::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

#[cfg(test)]
mod test {
    use super::*;
    use core::str::FromStr;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_basic_functionality() -> Result<()> {
        let temp_path = NamedTempFile::new().unwrap().into_temp_path();
        let db = SqliteStorage::new(temp_path.to_path_buf()).await?;

        let r = Resource::from("1");
        let a = Action::from("2");
        let e = Expr::from_str("345")?;
        db.set_policy(&r, &a, &e).await?;
        assert!(
            db.get_policy(&r, &a).await?.unwrap().equals(&e)?,
            "Verify set and get"
        );

        let policies = db.policies(&r).await?;
        assert_eq!(policies.len(), 1);

        let a = Action::from("3");
        db.set_policy(&r, &a, &e).await?;
        let policies = db.policies(&r).await?;
        assert_eq!(policies.len(), 2);

        db.del_policy(&r, &a).await?;
        let policies = db.policies(&r).await?;
        assert_eq!(policies.len(), 1);

        Ok(())
    }
}
