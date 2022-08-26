use anyhow::Context;
use cli_table::{Cell, Style, Table};
use core::fmt::Write;
use ockam::identity::credential::Credential;
use ockam_api::cloud::project::{Enroller, Project};

use crate::util::comma_separated;
use ockam_api::cloud::space::Space;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_api::route_to_multiaddr;
use ockam_core::route;
use std::cmp;

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

impl Output for Vec<Project<'_>> {
    fn output(&self) -> anyhow::Result<String> {
        let mut rows = Vec::new();
        for p in self {
            rows.push([p.output()?.cell()])
        }
        let table = rows
            .table()
            .title(["Projects".cell().bold(true)])
            .display()?
            .to_string();
        Ok(table)
    }
}

#[rustfmt::skip]
impl Output for Project<'_> {
    fn output(&self) -> anyhow::Result<String> {
        const MAX_WIDTH: usize = 80;
        const SPACES: usize = 4;

        let mut w = String::new();
        write!(w, "Project")?;
        write!(w, "\n  Id: {}", self.id)?;

        let prefix = "  Name: ";
        let value: Vec<char> = self.name.chars().collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Users: ";
        let value: Vec<char> = comma_separated(&self.users).chars().collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Services: ";
        let value: Vec<char> = comma_separated(&self.services).chars().collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Access route: ";
        let value: Vec<char> = self.access_route.to_string().chars().collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Identity identifier: ";
        let value: Vec<char> = self
            .identity
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| "N/A".to_string())
            .chars()
            .collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Authority access route: ";
        let value: Vec<char> = self
            .authority_access_route
            .as_deref()
            .unwrap_or("N/A")
            .chars()
            .collect();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

        let prefix = "  Authority identity: ";
        let value: Vec<char> = self
            .authority_identity
            .as_ref()
            .map(|b| b.to_string())
            .unwrap_or_else(|| "N/A".to_string())
            .chars()
            .collect::<Vec<char>>();
        write!(w, "\n{prefix}{}", wrapped_string(prefix.len(), SPACES, MAX_WIDTH, &value))?;

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

/// A naive way to ensure some string is not exceeding a certain width.
///
/// This function takes a char slice and turns it into a string with
/// interspersed newlines. No newline is more that prefix + max characters
/// from the previous one.
///
/// # Panics
///
/// - If spaces >= max
/// - If prefix >= max
fn wrapped_string(prefix: usize, spaces: usize, max: usize, val: &[char]) -> String {
    assert!(prefix <= max);
    assert!(spaces <= max);
    let (first, rest) = val.split_at(cmp::min(val.len(), max - prefix));
    let mut out = String::from_iter(first.iter());
    let mut iter = rest.chunks(max - spaces).peekable();
    let whitespace = std::iter::repeat(' ').take(spaces);
    if iter.peek().is_some() {
        out += "\n";
        out.extend(whitespace.clone())
    }
    while let Some(line) = iter.next() {
        out += &String::from_iter(line.iter().copied());
        if iter.peek().is_some() {
            out += "\n";
            out.extend(whitespace.clone())
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, TestResult};

    quickcheck! {
        fn prop_wrapped_string(prefix: usize, spaces: usize, max: usize, input: String) -> TestResult {
            if spaces >= max || prefix >= max {
                return TestResult::discard()
            }
            let chars: Vec<char> = input.chars().collect();
            let output = super::wrapped_string(prefix, spaces, max, &chars);
            TestResult::from_bool(output.split_whitespace().all(|c| c.chars().count() <= max))
        }
    }
}
