use anyhow::Context;
use cli_table::{Cell, Style, Table};
use core::fmt::Write;
use ockam::identity::credential::Credential;
use ockam_api::cloud::project::Project;

use ockam_api::nodes::models::portal::OutletStatus;

use crate::project::ProjectInfo;
use crate::util::comma_separated;
use crate::Result;
use colorful::Colorful;
use ockam_api::cloud::space::Space;
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelResponse, ShowSecureChannelResponse,
};
use ockam_api::route_to_multiaddr;
use ockam_core::route;

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
///     fn output(&self) -> Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output {
    fn output(&self) -> Result<String>;
}

impl<O: Output> Output for &O {
    fn output(&self) -> Result<String> {
        (*self).output()
    }
}

impl Output for Space<'_> {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Space")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Users: {}", comma_separated(&self.users))?;
        Ok(w)
    }
}

impl Output for Vec<Space<'_>> {
    fn output(&self) -> Result<String> {
        if self.is_empty() {
            return Ok("No spaces found".to_string());
        }
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
                "Id".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for Project {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "Project")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Access route: {}", self.access_route)?;
        write!(
            w,
            "\n  Identity identifier: {}",
            self.identity
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_default()
        )?;
        write!(
            w,
            "\n  Version: {}",
            self.version.as_deref().unwrap_or("N/A")
        )?;
        write!(w, "\n  Running: {}", self.running.unwrap_or(false))?;
        Ok(w)
    }
}

impl Output for ProjectInfo<'_> {
    fn output(&self) -> Result<String> {
        let pi = self
            .identity
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let ar = self.authority_access_route.as_deref().unwrap_or("N/A");
        let ai = self.authority_identity.as_deref().unwrap_or("N/A");
        let mut w = String::new();
        writeln!(w, "{}: {}", "Project ID".bold(), self.id)?;
        writeln!(w, "{}: {}", "Project identity".bold(), pi)?;
        writeln!(w, "{}: {}", "Authority address".bold(), ar)?;
        write!(w, "{}: {}", "Authority identity".bold(), ai)?;
        Ok(w)
    }
}

impl Output for Vec<Project> {
    fn output(&self) -> Result<String> {
        if self.is_empty() {
            return Ok("No projects found".to_string());
        }
        let mut rows = vec![];
        for Project {
            id,
            name,
            users,
            space_name,
            ..
        } in self
        {
            rows.push([
                id.cell(),
                name.cell(),
                comma_separated(users).cell(),
                space_name.cell(),
            ]);
        }
        let table = rows
            .table()
            .title([
                "Id".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
                "Space Name".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for CreateSecureChannelResponse<'_> {
    fn output(&self) -> Result<String> {
        let addr = route_to_multiaddr(&route![self.addr.to_string()])
            .context("Invalid Secure Channel Address")?
            .to_string();
        Ok(addr)
    }
}

impl Output for ShowSecureChannelResponse<'_> {
    fn output(&self) -> Result<String> {
        let s = match &self.channel {
            Some(addr) => {
                format!(
                    "\n  Secure Channel:\n{} {}\n{} {}\n{} {}",
                    "  •         At: ".light_magenta(),
                    route_to_multiaddr(&route![addr.to_string()])
                        .context("Invalid Secure Channel Address")?
                        .to_string()
                        .light_yellow(),
                    "  •         To: ".light_magenta(),
                    self.route.as_ref().unwrap().light_yellow(),
                    "  • Authorized: ".light_magenta(),
                    self.authorized_identifiers
                        .as_ref()
                        .unwrap_or(&Vec::<ockam_core::CowStr>::from(["none".into()]))
                        .iter()
                        .map(|id| id.light_yellow().to_string())
                        .collect::<Vec<String>>()
                        .join("\n\t")
                )
            }
            None => format!("{}", "Channel not found".red()),
        };

        Ok(s)
    }
}

impl Output for OutletStatus<'_> {
    fn output(&self) -> Result<String> {
        let output = format!(
            r#"
Outlet {}:
    TCP Address:    {}
    Worker Address: {}
"#,
            self.alias,
            self.tcp_addr,
            self.worker_address()?
        );

        Ok(output)
    }
}
impl Output for Credential {
    fn output(&self) -> Result<String> {
        Ok(self.to_string())
    }
}

impl Output for Vec<u8> {
    fn output(&self) -> Result<String> {
        Ok(hex::encode(self))
    }
}
