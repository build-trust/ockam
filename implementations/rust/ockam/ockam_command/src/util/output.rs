use anyhow::Context;
use cli_table::{Cell, Style, Table};
use core::fmt::Write;
use ockam::identity::credential::Credential;
use ockam_api::cloud::project::{Enroller, Project};

use crate::util::comma_separated;
use crate::CommandGlobalOpts;
use ockam_api::cloud::space::Space;
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelResponse, DeleteSecureChannelResponse,
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
///     fn output(&self) -> anyhow::Result<String> {
///         Ok(self.to_string())
///     }
/// }
/// ```
pub trait Output: serde::Serialize {
    fn output(&self) -> anyhow::Result<String>;

    fn print(&self, opts: &CommandGlobalOpts) -> crate::Result<()> {
        let o = match opts.global_args.output_format {
            crate::OutputFormat::Plain => {
                self.output().context("Failed to serialize response body")?
            }
            crate::OutputFormat::Json => {
                serde_json::to_string_pretty(&self).context("Failed to serialize response body")?
            }
        };
        println!("{}", o);
        Ok(())
    }
}

impl Output for Space<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Space")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Users: {}", comma_separated(&self.users))?;
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
                "Id".cell().bold(true),
                "Name".cell().bold(true),
                "Users".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for Project<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Project")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Name: {}", self.name)?;
        write!(w, "\n  Users: {}", comma_separated(&self.users))?;
        write!(w, "\n  Services: {}", comma_separated(&self.services))?;
        write!(w, "\n  Access route: {}", self.access_route)?;
        write!(w, "\n  Identity: {:?}", self.identity)?;
        Ok(w)
    }
}

impl Output for CreateSecureChannelResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let addr = route_to_multiaddr(&route![self.addr.to_string()])
            .context("Invalid Secure Channel Address")?
            .to_string();
        Ok(addr)
    }
}

impl Output for Enroller<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Enroller")?;
        write!(w, "\n  Identity id: {}", self.identity_id)?;
        write!(w, "\n  Added by: {}", self.added_by)?;
        Ok(w)
    }
}

impl Output for Vec<Enroller<'_>> {
    fn output(&self) -> anyhow::Result<String> {
        let mut rows = vec![];
        for Enroller {
            identity_id,
            added_by,
            ..
        } in self
        {
            rows.push([identity_id.cell(), added_by.cell()]);
        }
        let table = rows
            .table()
            .title([
                "Identity ID".cell().bold(true),
                "Added By".cell().bold(true),
            ])
            .display()?
            .to_string();
        Ok(table)
    }
}

impl Output for Credential<'_> {
    fn output(&self) -> anyhow::Result<String> {
        Ok(self.to_string())
    }
}

impl Output for DeleteSecureChannelResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        match &self.channel {
            Some(ch) => write!(w, "deleted: {}", ch)?,
            None => write!(w, "channel not found")?,
        }
        Ok(w)
    }
}

impl Output for Vec<Project<'_>> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Output for Vec<Project<'_>> : not implemented")?;
        Ok(w)
    }
}
