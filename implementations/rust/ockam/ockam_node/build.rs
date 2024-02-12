fn main() {
    // Trigger a recompile if the migrations directory has changed.
    // https://docs.rs/sqlx/latest/sqlx/macro.migrate.html#triggering-recompilation-on-migration-changes
    println!("cargo:rerun-if-changed=src/storage/database/migrations");
}
