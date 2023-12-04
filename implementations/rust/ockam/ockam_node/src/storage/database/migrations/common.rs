use crate::database::SqlxDatabase;

impl SqlxDatabase {
    pub(super) fn table_exists(table_name: &str) -> String {
        format!("SELECT EXISTS(SELECT name FROM sqlite_schema WHERE type = 'table' AND name = '{table_name}')")
    }
}
