use core::str;
use ockam_core::async_trait;
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_node::tokio::task::{self, JoinError};
use rusqlite::{params, Connection};
use std::fmt;
use std::path::Path;
use tokio_retry::strategy::{jitter, FixedInterval};
use tokio_retry::Retry;
use tracing::debug;

use Storage;

/// Storage using the Sqlite database
#[derive(Clone)]
pub struct SqliteStorage {
    /// Sqlite Connection
    conn: Arc<Mutex<Connection>>,
}

impl fmt::Debug for SqliteStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SqliteStore")
    }
}

impl SqliteStorage {
    const CREATE_IDENTITY_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS identity (
        id INTEGER PRIMARY KEY,
        identity_id TEXT NOT NULL,
        key TEXT NOT NULL,
        value BLOB
    );";
    const CREATE_IDENTITY_INDEX_SQL: &str =
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_identity_id_key ON identity (identity_id, key);";

    const CREATE_POLICY_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS policy (
        id INTEGER PRIMARY KEY,
        resource TEXT NOT NULL,
        action TEXT NOT NULL,
        value BLOB
    );";
    const CREATE_POLICY_INDEX_SQL: &str = "CREATE UNIQUE INDEX IF NOT EXISTS idx_policy_resource_action ON policy (resource, action);";

    /// Constructor
    pub async fn new<P: AsRef<Path>>(p: P) -> Result<Self> {
        // Not sure we need this
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
        debug!("create the Sqlite database");
        let p = p.to_path_buf();
        // Creates database file if it doesn't exist
        let conn = Connection::open(p).map_err(map_sqlite_err)?;
        let _ = conn
            .execute_batch(
                &("PRAGMA encoding = 'UTF-8';".to_owned()
                    + SqliteStorage::CREATE_IDENTITY_TABLE_SQL
                    + SqliteStorage::CREATE_IDENTITY_INDEX_SQL
                    + SqliteStorage::CREATE_POLICY_TABLE_SQL
                    + SqliteStorage::CREATE_POLICY_INDEX_SQL),
            )
            .map_err(map_sqlite_err)?;
        Ok(SqliteStorage {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Getter for Sqlite Connection
    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn get(&self, id: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.conn();
        let id = String::from(id);
        let key = String::from(key);

        let t = move || {
            let conn = conn.lock().unwrap();
            let result = conn
                .query_row::<Vec<u8>, _, _>(
                    "SELECT value FROM identity WHERE identity_id = ?1 AND key = ?2;",
                    params![id, key],
                    |row| row.get(0),
                )
                .map_err(map_sqlite_err)?;
            Ok(Some(result))
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn set(&self, id: &str, key: String, val: Vec<u8>) -> Result<()> {
        let conn = self.conn();
        let id = String::from(id);
        let t = move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO identity (identity_id, key, value) VALUES (?1, ?2, ?3)",
                params![id, key, val],
            )
            .map_err(map_sqlite_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn del(&self, id: &str, key: &str) -> Result<()> {
        let conn = self.conn();
        let id = String::from(id);
        let key = String::from(key);
        let t = move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                "DELETE FROM identity WHERE identity_id = ?1 AND key = ?2;",
                params![id, key],
            )
            .map_err(map_sqlite_err)?;
            Ok(())
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }

    async fn keys(&self, namespace: &str) -> Result<Vec<String>> {
        let conn = self.conn();
        let namespace = String::from(namespace);
        let t = move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT identity_id FROM identity WHERE key = ?1;")
                .map_err(map_sqlite_err)?;
            let result: Result<Vec<String>> = stmt
                .query_map(params![namespace], |row| row.get(0))
                .map_err(map_sqlite_err)?
                .map(|value| value.map_err(map_sqlite_err))
                .collect();
            result
        };
        task::spawn_blocking(t).await.map_err(map_join_err)?
    }
}

fn map_join_err(err: JoinError) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

fn map_sqlite_err(err: rusqlite::Error) -> Error {
    Error::new(Origin::Application, Kind::Io, err)
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_basic_functionality() -> Result<()> {
        let temp_path = NamedTempFile::new().unwrap().into_temp_path();
        let db = SqliteStorage::new(temp_path.to_path_buf()).await?;

        db.set("1", String::from("2"), vec![1, 2, 3, 4]).await?;
        assert_eq!(
            db.get("1", "2").await?,
            Some(vec![1, 2, 3, 4]),
            "Verify set and get"
        );
        assert_eq!(db.keys("2").await?.len(), 1, "Verify keys");

        db.set("2", String::from("2"), vec![1, 2, 3, 4]).await?;
        assert_eq!(db.keys("2").await?.len(), 2, "Verify multiple keys");

        db.del("2", "2").await?;
        assert_eq!(db.keys("2").await?.len(), 1, "Verify delete");

        Ok(())
    }
}
