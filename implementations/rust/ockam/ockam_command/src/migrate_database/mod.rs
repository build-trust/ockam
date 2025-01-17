use crate::docs;
use crate::util::async_cmd;
use crate::CommandGlobalOpts;
use clap::Args;
use miette::miette;
use ockam_node::database::node_migration_set::NodeMigrationSet;
use ockam_node::database::{DatabaseConfiguration, MigrationSet, SqlxDatabase};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
    hide = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct MigrateDatabaseCommand {
    /// Report which migrations would be applied, but don't apply them
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
}

impl MigrateDatabaseCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "migrate_database".into()
    }

    /// If a Postgres database is accessible, either:
    ///   - Get the current migration status with the `--dry-run` option,
    ///   - Or execute all the possible migrations and return the status after migration.
    ///
    /// This command returns true when used in scripts if the command successfully executed.
    async fn async_run(&self, _ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        match DatabaseConfiguration::postgres()? {
            Some(configuration) => {
                let db = SqlxDatabase::create_no_migration(&configuration).await?;
                let migration_set = NodeMigrationSet::new(configuration.database_type());
                let migrator = migration_set.create_migrator()?;
                if !self.dry_run {
                    migrator.migrate(&db.pool).await?;
                };

                let status = migrator.migration_status(&db.pool).await?;
                opts.terminal.stdout().plain(&status).json_obj(&status)?.machine(status.up_to_date()).write_line()?;

                Ok(())
            },
            None => Err(miette!("There is no postgres database configuration, or it is incomplete. Please run ockam environment to check the database environment variables")),
        }
    }
}
