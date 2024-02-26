use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::compat::time::now;
use ockam_core::errcode::{Kind, Origin};
use sqlx::migrate::{Migrate, Migration};
use sqlx::sqlite::SqliteRow;
use sqlx::{query, Row, SqliteConnection, SqlitePool};
use std::cmp::Ordering;
use time::OffsetDateTime;

use crate::database::migrations::migration_support::rust_migration::RustMigration;
use crate::database::{FromSqlxError, ToSqlxType, ToVoid};
use ockam_core::Result;

/// Migrator is responsible for running Sql and Rust migrations side by side in the correct order,
/// checking for conflicts, duplicates; making sure each migration runs only once
pub struct Migrator {
    // Unsorted, no duplicates
    rust_migrations: Vec<Box<dyn RustMigration>>,
    // Unsorted, no duplicates
    sql_migrator: sqlx::migrate::Migrator,
}

impl Migrator {
    /// Constructor
    pub fn new(sql_migrator: sqlx::migrate::Migrator) -> Result<Self> {
        let iter = sql_migrator.iter().map(|m| m.version);

        Self::check_duplicates(iter)?;

        Ok(Self {
            rust_migrations: vec![],
            sql_migrator,
        })
    }

    fn check_duplicates(iter: impl Iterator<Item = i64>) -> Result<()> {
        let mut versions = HashSet::new();

        for version in iter {
            let duplicate = !versions.insert(version);

            if duplicate {
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Conflict,
                    format!("Found duplicate migration version: {}", version),
                ));
            }
        }

        Ok(())
    }

    /// Set rust migration
    pub fn set_rust_migrations(
        &mut self,
        rust_migrations: Vec<Box<dyn RustMigration>>,
    ) -> Result<()> {
        let iter = rust_migrations.iter().map(|m| m.version());

        Self::check_duplicates(iter)?;

        self.rust_migrations = rust_migrations;

        Ok(())
    }
}

impl Migrator {
    fn is_inside_version_range(
        version: i64,
        from_version: i64, // including
        to_version: i64,
        including_to: bool,
    ) -> bool {
        if from_version <= version && version < to_version {
            return true;
        }

        if version == to_version && including_to {
            return true;
        }

        false
    }

    async fn run_migrations(
        &self,
        connection: &mut SqliteConnection,
        from_version: i64,  // including
        to_version: i64,    // not including
        run_last_sql: bool, // will run sql migration with verion == to_version
    ) -> Result<()> {
        connection.ensure_migrations_table().await.into_core()?;

        let version = connection.dirty_version().await.into_core()?;
        if let Some(version) = version {
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::Conflict,
                format!("Sql migration previously failed for version {}", version),
            ));
        }

        // sqlx Migrator also optionally checks for missing migrations (ones that had been run and
        // marked as migrated in the db but now don't exist). Skipping that check for now.
        // WARNING: the check if re-enabled can potentially fail because of renaming
        // 20240111100000_add_rust_migrations.sql -> 20231230100000_add_rust_migrations.sql
        // which was needed to track rust migrations that were added
        // before the _rust_migrations table existed
        let applied_migrations = connection.list_applied_migrations().await.into_core()?;

        let applied_migrations: HashMap<_, _> = applied_migrations
            .into_iter()
            .map(|m| (m.version, m))
            .collect();

        enum NextMigration<'a> {
            Sql(&'a Migration),
            #[allow(clippy::borrowed_box)]
            Rust(&'a Box<dyn RustMigration>),
        }

        impl NextMigration<'_> {
            fn is_sql(&self) -> bool {
                match self {
                    NextMigration::Sql(_) => true,
                    NextMigration::Rust(_) => false,
                }
            }
        }

        let sql_iterator = self.sql_migrator.migrations.iter().filter_map(|m| {
            let version = m.version;

            if !Self::is_inside_version_range(version, from_version, to_version, run_last_sql) {
                return None;
            }

            Some((version, NextMigration::Sql(m)))
        });
        let rust_iterator = self.rust_migrations.iter().filter_map(|m| {
            let version = m.version();

            if !Self::is_inside_version_range(version, from_version, to_version, false) {
                return None;
            }

            Some((version, NextMigration::Rust(m)))
        });

        let mut all_migrations: Vec<(i64, NextMigration)> =
            sql_iterator.chain(rust_iterator).collect();
        all_migrations.sort_by(|m1, m2| match m1.0.cmp(&m2.0) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => {
                // Sql migrations go first
                if m1.1.is_sql() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            Ordering::Greater => Ordering::Greater,
        });

        for migration in all_migrations.iter().map(|(_version, m)| m) {
            match migration {
                NextMigration::Sql(sql_migration) => {
                    if sql_migration.migration_type.is_down_migration() {
                        return Ok(());
                    }

                    match applied_migrations.get(&sql_migration.version) {
                        Some(applied_migration) => {
                            if sql_migration.checksum != applied_migration.checksum {
                                return Err(ockam_core::Error::new(
                                    Origin::Node,
                                    Kind::Conflict,
                                    format!(
                                        "Checksum mismatch for sql migration for version {}",
                                        sql_migration.version
                                    ),
                                ));
                            }
                        }
                        None => {
                            connection.apply(sql_migration).await.into_core()?;
                        }
                    }
                }
                NextMigration::Rust(rust_migration) => {
                    if Self::has_migrated(connection, rust_migration.name()).await? {
                        continue;
                    }
                    if rust_migration.migrate(connection).await? {
                        Self::mark_as_migrated(connection, rust_migration.name()).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Migrator {
    pub(crate) async fn has_migrated(
        connection: &mut SqliteConnection,
        migration_name: &str,
    ) -> Result<bool> {
        let query = query("SELECT COUNT(*) FROM _rust_migrations WHERE name=?")
            .bind(migration_name.to_sql());
        let count_raw: Option<SqliteRow> =
            query.fetch_optional(&mut *connection).await.into_core()?;

        if let Some(count_raw) = count_raw {
            let count: i64 = count_raw.get(0);
            Ok(count != 0)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn mark_as_migrated(
        connection: &mut SqliteConnection,
        migration_name: &str,
    ) -> Result<()> {
        let now = now()?;
        let now = OffsetDateTime::from_unix_timestamp(now as i64).map_err(|_| {
            ockam_core::Error::new(Origin::Node, Kind::Internal, "Can't convert timestamp")
        })?;
        let query = query("INSERT OR REPLACE INTO _rust_migrations (name, run_on) VALUES (?, ?)")
            .bind(migration_name.to_sql())
            .bind(now.to_sql());
        query.execute(&mut *connection).await.void()?;

        Ok(())
    }
}

impl Migrator {
    /// Run migrations
    pub async fn migrate_partial(
        &self,
        pool: &SqlitePool,
        from_version: i64,  // including
        to_version: i64,    // not including
        run_last_sql: bool, // Will run `to_version` version of the sql migration
    ) -> Result<()> {
        let mut connection = pool.acquire().await.into_core()?;

        // Apparently does nothing for sqlite...
        connection.lock().await.into_core()?;

        let res = self
            .run_migrations(&mut connection, from_version, to_version, run_last_sql)
            .await;

        connection.unlock().await.into_core()?;

        res?;

        Ok(())
    }

    /// Run all migrations
    pub async fn migrate(&self, pool: &SqlitePool) -> Result<()> {
        self.migrate_partial(pool, 0, i64::MAX, false).await
    }
}

#[cfg(test)]
impl Migrator {
    /// Migrate the schema of the database right before the specified version
    pub(crate) async fn migrate_before(
        &self,
        pool: &SqlitePool,
        version: i64, // not including
        run_last_sql: bool,
    ) -> Result<()> {
        self.migrate_partial(pool, 0, version, run_last_sql).await
    }
}
