use crate::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::compat::time::now;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use sqlx::sqlite::SqliteRow;
use sqlx::{query, Row, SqlitePool};
use time::OffsetDateTime;

impl SqlxDatabase {
    pub(crate) async fn has_migrated(pool: &SqlitePool, migration_name: &str) -> Result<bool> {
        let query = query("SELECT COUNT(*) FROM _rust_migrations WHERE name=?")
            .bind(migration_name.to_sql());
        let count_raw: Option<SqliteRow> = query.fetch_optional(pool).await.into_core()?;

        if let Some(count_raw) = count_raw {
            let count: i64 = count_raw.get(0);
            Ok(count != 0)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn mark_as_migrated(pool: &SqlitePool, migration_name: &str) -> Result<()> {
        let now = now()?;
        let now = OffsetDateTime::from_unix_timestamp(now as i64).map_err(|_| {
            ockam_core::Error::new(Origin::Node, Kind::Internal, "Can't convert timestamp")
        })?;
        let query = query("INSERT OR REPLACE INTO _rust_migrations (name, run_on) VALUES (?, ?)")
            .bind(migration_name.to_sql())
            .bind(now.to_sql());
        query.execute(pool).await.void()?;

        Ok(())
    }
}
