use crate::database::migrations::migration_support::migration_status::MigrationStatus;
use crate::database::migrations::migration_support::rust_migration::RustMigration;
use crate::database::MigrationResult::MigrationSuccess;
use crate::database::{FromSqlxError, MigrationFailure, MigrationResult, ToVoid};
use core::fmt::{Display, Formatter};
use ockam_core::compat::collections::HashSet;
use ockam_core::compat::time::now;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use serde::Serialize;
use sqlx::any::AnyRow;
use sqlx::migrate::{AppliedMigration, Migrate, Migration as SqlxMigration};
use sqlx::{query, Any, AnyConnection, Pool, Row};
use sqlx_core::executor::Executor;
use std::cmp::Ordering;
use time::OffsetDateTime;

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
        let iter = sql_migrator.iter().map(|m| Version(m.version));
        Self::check_duplicates(iter)?;

        Ok(Self {
            rust_migrations: vec![],
            sql_migrator,
        })
    }

    fn check_duplicates(iter: impl Iterator<Item = Version>) -> Result<()> {
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

enum Mode {
    DryRun,
    ApplyMigrations,
}

impl Migrator {
    async fn needs_migration(
        &self,
        connection: &mut AnyConnection,
        up_to: Version,
    ) -> Result<bool> {
        let status = self
            .run_migrations_impl(connection, up_to, Mode::DryRun)
            .await?;
        match status {
            MigrationStatus::UpToDate(_) => Ok(false),
            MigrationStatus::Todo(_, _) => Ok(true),
            MigrationStatus::Failed(version, reason) => Err(ockam_core::Error::new(
                Origin::Node,
                Kind::Conflict,
                format!(
                    "Sql migration previously failed for version {}. Reason: {}",
                    version, reason
                ),
            )),
        }
    }

    async fn run_migrations(
        &self,
        connection: &mut AnyConnection,
        up_to: Version,
    ) -> Result<MigrationStatus> {
        self.run_migrations_impl(connection, up_to, Mode::ApplyMigrations)
            .await
    }

    async fn run_migrations_impl(
        &self,
        connection: &mut AnyConnection,
        up_to: Version,
        mode: Mode,
    ) -> Result<MigrationStatus> {
        connection.ensure_migrations_table().await.into_core()?;

        let version = connection.dirty_version().await.into_core()?;
        if let Some(version) = version {
            return Ok(MigrationStatus::Failed(
                Version(version),
                MigrationFailure::DirtyVersion,
            ));
        }

        let migrations = {
            let sql_iterator = self.sql_migrator.migrations.iter().filter_map(|m| {
                if Version(m.version) <= up_to {
                    Some(NextMigration::Sql(m))
                } else {
                    None
                }
            });
            let rust_iterator = self.rust_migrations.iter().filter_map(|m| {
                if m.version() <= up_to {
                    Some(NextMigration::Rust(m.as_ref()))
                } else {
                    None
                }
            });
            let mut migrations: Vec<NextMigration> = sql_iterator.chain(rust_iterator).collect();
            migrations.sort();
            migrations
        };

        // sqlx Migrator also optionally checks for missing migrations (ones that had been run and
        // marked as migrated in the db but now don't exist). Skipping that check for now.
        // WARNING: the check if re-enabled can potentially fail because of renaming
        // 20240111100000_add_rust_migrations.sql -> 20231230100000_add_rust_migrations.sql
        // which was needed to track rust migrations that were added
        // before the _rust_migrations table existed
        let applied_migrations = connection.list_applied_migrations().await.into_core()?;
        let last_applied_migration = applied_migrations.last().map(|m| Version(m.version));
        let next_migration_to_apply = migrations
            .last()
            .map(|m| m.version())
            .unwrap_or(Version::MIN);

        match mode {
            Mode::DryRun => {
                let mut last_migrated_version = Version::MIN;
                for migration in migrations.into_iter() {
                    let needs_migration = match migration {
                        NextMigration::Sql(sql_migration) => {
                            NextMigration::needs_sql_migration(
                                sql_migration,
                                connection,
                                &applied_migrations,
                            )
                            .await?
                        }
                        NextMigration::Rust(rust_migration) => {
                            NextMigration::needs_rust_migration(
                                rust_migration,
                                connection,
                                &applied_migrations,
                            )
                            .await?
                        }
                    };

                    if needs_migration {
                        return Ok(MigrationStatus::Todo(
                            last_applied_migration,
                            next_migration_to_apply,
                        ));
                    };
                    last_migrated_version = migration.version();
                }
                Ok(MigrationStatus::UpToDate(last_migrated_version))
            }
            Mode::ApplyMigrations => {
                let mut last_migrated_version = Version::MIN;
                for migration in migrations.into_iter() {
                    match migration {
                        NextMigration::Sql(sql_migration) => {
                            match NextMigration::apply_sql_migration(
                                sql_migration,
                                connection,
                                &applied_migrations,
                            )
                            .await?
                            {
                                MigrationSuccess => (),
                                MigrationResult::MigrationFailure(failure) => {
                                    return Ok(MigrationStatus::Failed(
                                        Version(sql_migration.version),
                                        failure,
                                    ))
                                }
                            }
                        }
                        NextMigration::Rust(rust_migration) => {
                            match NextMigration::apply_rust_migration(rust_migration, connection)
                                .await?
                            {
                                MigrationSuccess => (),
                                MigrationResult::MigrationFailure(failure) => {
                                    return Ok(MigrationStatus::Failed(
                                        rust_migration.version(),
                                        failure,
                                    ))
                                }
                            }
                        }
                    }
                    last_migrated_version = migration.version();
                }
                Ok(MigrationStatus::UpToDate(last_migrated_version))
            }
        }
    }
}

impl Migrator {
    pub(crate) async fn has_migrated(
        connection: &mut AnyConnection,
        migration_name: &str,
    ) -> Result<bool> {
        let query =
            query("SELECT COUNT(*) FROM _rust_migrations WHERE name = $1").bind(migration_name);
        let count_raw: Option<AnyRow> = query.fetch_optional(&mut *connection).await.into_core()?;

        if let Some(count_raw) = count_raw {
            let count: i64 = count_raw.get(0);
            Ok(count != 0)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn mark_as_migrated(
        connection: &mut AnyConnection,
        migration_name: &str,
    ) -> Result<()> {
        let now = now()?;
        let now = OffsetDateTime::from_unix_timestamp(now as i64).map_err(|_| {
            ockam_core::Error::new(Origin::Node, Kind::Internal, "Can't convert timestamp")
        })?;
        let query = query(
            r#"
            INSERT INTO _rust_migrations (name, run_on)
            VALUES ($1, $2)
            ON CONFLICT (name)
            DO UPDATE SET run_on = $2"#,
        )
        .bind(migration_name)
        .bind(now.unix_timestamp());
        query.execute(&mut *connection).await.void()?;

        Ok(())
    }
}

impl Migrator {
    /// Run migrations up to the specified version (inclusive)
    pub(crate) async fn migrate_up_to(
        &self,
        pool: &Pool<Any>,
        up_to: Version,
    ) -> Result<MigrationStatus> {
        let mut connection = pool.acquire().await.into_core()?;

        if !self.needs_migration(&mut connection, up_to).await? {
            debug!("No database migrations was required");
            return Ok(MigrationStatus::UpToDate(up_to));
        }

        let is_sqlite = connection.backend_name() == "SQLite";
        if is_sqlite {
            debug!("Migrating SQLite database with exclusive locking");
            connection
                .execute("PRAGMA locking_mode = EXCLUSIVE;")
                .await
                .into_core()?;
        } else {
            // This lock is only effective for Postgres
            connection.lock().await.into_core()?;
        };

        let result = self.run_migrations(&mut connection, up_to).await;
        if is_sqlite {
            debug!("Migration completed, unlocking database");
            // This is not enough to unlock the database, according to the documentation,
            // we also need an arbitrary read or write operation to release the
            // exclusive lock.
            // In practice, we are closing the connection to release the lock.
            connection
                .execute("PRAGMA locking_mode = NORMAL;")
                .await
                .into_core()?;
        } else {
            connection.unlock().await.into_core()?;
        }
        result
    }

    /// Run all migrations
    pub async fn migrate(&self, pool: &Pool<Any>) -> Result<MigrationStatus> {
        self.migrate_up_to(pool, Version::MAX).await
    }

    /// Return the migration status
    pub async fn migration_status(&self, pool: &Pool<Any>) -> Result<MigrationStatus> {
        let mut connection = pool.acquire().await.into_core()?;
        self.run_migrations_impl(&mut connection, Version::MAX, Mode::DryRun)
            .await
    }
}

#[cfg(test)]
impl Migrator {
    /// Run migrations up to the specified version (inclusive) but skip the last rust migration
    pub(crate) async fn migrate_up_to_skip_last_rust_migration(
        mut self,
        pool: &Pool<Any>,
        up_to: Version,
    ) -> Result<MigrationStatus> {
        self.rust_migrations.retain(|m| m.version() < up_to);
        self.migrate_up_to(pool, up_to).await
    }
}

/// Migration version
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Version(pub i64);

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Version {
    const MIN: Version = Version(0);
    const MAX: Version = Version(i64::MAX);
}

#[derive(Debug)]
enum NextMigration<'a> {
    Sql(&'a SqlxMigration),
    Rust(&'a dyn RustMigration),
}

impl NextMigration<'_> {
    fn is_sql(&self) -> bool {
        matches!(self, Self::Sql(_))
    }

    fn version(&self) -> Version {
        match self {
            Self::Sql(m) => Version(m.version),
            Self::Rust(m) => m.version(),
        }
    }

    async fn needs_sql_migration<'a>(
        migration: &'a SqlxMigration,
        _connection: &mut AnyConnection,
        applied_migrations: &[AppliedMigration],
    ) -> Result<bool> {
        if migration.migration_type.is_down_migration() {
            return Ok(false);
        }
        match applied_migrations
            .iter()
            .find(|m| m.version == migration.version)
        {
            Some(applied_migration) => {
                if migration.checksum != applied_migration.checksum {
                    return Err(ockam_core::Error::new(
                        Origin::Node,
                        Kind::Conflict,
                        format!(
                            "Checksum mismatch for sql migration '{}' for version {}",
                            migration.description, migration.version,
                        ),
                    ));
                }
                Ok(false)
            }
            None => Ok(true),
        }
    }

    async fn apply_sql_migration<'a>(
        migration: &'a SqlxMigration,
        connection: &mut AnyConnection,
        applied_migrations: &[AppliedMigration],
    ) -> Result<MigrationResult> {
        if migration.migration_type.is_down_migration() {
            return Ok(MigrationResult::down_migration());
        }
        match applied_migrations
            .iter()
            .find(|m| m.version == migration.version)
        {
            Some(applied_migration) => {
                if migration.checksum != applied_migration.checksum {
                    Ok(MigrationResult::incorrect_checksum(
                        migration.description.to_string(),
                        migration.sql.to_string(),
                        String::from_utf8(migration.checksum.to_vec())
                            .unwrap_or("actual migration checksum cannot be displayed".to_string()),
                        String::from_utf8(migration.checksum.to_vec()).unwrap_or(
                            "expected migration checksum cannot be displayed".to_string(),
                        ),
                    ))
                } else {
                    Ok(MigrationSuccess)
                }
            }
            None => match connection.apply(migration).await.into_core() {
                Ok(_) => Ok(MigrationSuccess),
                Err(e) => Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Conflict,
                    format!(
                        "Failed to run the migration {}: {e:?}",
                        migration.description
                    ),
                )),
            },
        }
    }

    async fn needs_rust_migration<'a>(
        migration: &'a dyn RustMigration,
        connection: &mut AnyConnection,
        _applied_migrations: &[AppliedMigration],
    ) -> Result<bool> {
        Ok(!Migrator::has_migrated(connection, migration.name()).await?)
    }

    async fn apply_rust_migration(
        migration: &dyn RustMigration,
        connection: &mut AnyConnection,
    ) -> Result<MigrationResult> {
        if Migrator::has_migrated(connection, migration.name()).await? {
            return Ok(MigrationResult::success());
        }
        if migration.migrate(connection).await? {
            Migrator::mark_as_migrated(connection, migration.name()).await?;
        }
        Ok(MigrationSuccess)
    }
}

impl Eq for NextMigration<'_> {}

impl PartialEq<Self> for NextMigration<'_> {
    fn eq(&self, other: &Self) -> bool {
        let same_kind = matches!(
            (self, other),
            (Self::Sql(_), Self::Sql(_)) | (Self::Rust(_), Self::Rust(_))
        );
        same_kind && self.version() == other.version()
    }
}

impl Ord for NextMigration<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.version().cmp(&other.version()) {
            Ordering::Equal => {
                // Sql migrations go first
                match (self.is_sql(), other.is_sql()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => unreachable!(),
                }
            }
            ord => ord,
        }
    }
}

impl PartialOrd for NextMigration<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::async_trait;
    use sqlx::migrate::MigrationType;

    #[test]
    fn ordering_of_migrations() {
        let sql_1 = SqlxMigration::new(1, "sql_1".into(), MigrationType::Simple, "1".into(), true);
        let sql_2 = SqlxMigration::new(2, "sql_2".into(), MigrationType::Simple, "2".into(), true);
        let rust_1: Box<dyn RustMigration> = Box::new(DummyRustMigration::new(Version(1)));
        let rust_2: Box<dyn RustMigration> = Box::new(DummyRustMigration::new(Version(2)));
        let rust_3: Box<dyn RustMigration> = Box::new(DummyRustMigration::new(Version(3)));

        let mut migrations = vec![
            NextMigration::Sql(&sql_2),
            NextMigration::Sql(&sql_1),
            NextMigration::Rust(rust_1.as_ref()),
            NextMigration::Rust(rust_3.as_ref()),
            NextMigration::Rust(rust_2.as_ref()),
        ];
        migrations.sort();

        for m in &migrations {
            match m {
                NextMigration::Sql(_) => {
                    assert!(m.is_sql());
                }
                NextMigration::Rust(_) => {
                    assert!(!m.is_sql());
                }
            }
        }

        assert_eq!(
            migrations,
            vec![
                NextMigration::Sql(&sql_1),
                NextMigration::Rust(rust_1.as_ref()),
                NextMigration::Sql(&sql_2),
                NextMigration::Rust(rust_2.as_ref()),
                NextMigration::Rust(rust_3.as_ref())
            ]
        );
    }

    #[derive(Debug)]
    struct DummyRustMigration {
        version: Version,
    }

    impl DummyRustMigration {
        fn new(version: Version) -> Self {
            Self { version }
        }
    }

    #[async_trait]
    impl RustMigration for DummyRustMigration {
        fn name(&self) -> &str {
            "DummyRustMigration"
        }

        fn version(&self) -> Version {
            self.version
        }

        async fn migrate(&self, _connection: &mut AnyConnection) -> Result<bool> {
            Ok(true)
        }
    }
}
