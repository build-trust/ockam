use cli_table::{Cell, Style, Table};
use core::fmt::Write;

use crate::util::comma_separated;
use ockam_api::cloud::space::Space;

/// Trait to control how a given type will be printed as a CLI output.
///
/// The `Output` allows us to reuse the same formatting logic across different commands
/// and extract the formatting logic out of the commands logic.
///
/// Note that we can't just implement the `Display` trait because most of the types we want
/// to output in the commands are defined in other crates. We can still reuse the `Display`
/// implementation if it's available and already formats the type as we want. For example:
///
/// ```ignore
/// struct MyType;
///
/// impl std::fmt::Display for MyType {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "MyType")
///     }
/// }
///
/// impl Output for MyType {
///     fn output(&self) -> anyhow::Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output {
    fn output(&self) -> anyhow::Result<String>;
}

impl Output for Space<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "id: {}", self.id)?;
        write!(w, "\nname: {}", self.name)?;
        write!(w, "\nusers: {}", comma_separated(&self.users))?;
        Ok(w)
    }
}

impl Output for Vec<Space<'_>> {
    fn output(&self) -> anyhow::Result<String> {
        let mut rows = vec![];
        for Space {
            id, name, users, ..
        } in self
        {
            rows.push([id.cell(), name.cell(), comma_separated(users).cell()]);
        }
        let table = rows
            .table()
            .title([
                "ID".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}
