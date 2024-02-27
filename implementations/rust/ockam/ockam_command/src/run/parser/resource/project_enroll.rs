use crate::project::EnrollCommand;
use crate::run::parser::resource::traits::ConfigRunner;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::PreRunHooks;
use crate::{color_primary, fmt_info, Command, CommandGlobalOpts, OckamSubcommand};
use async_trait::async_trait;
use colorful::Colorful;
use miette::{miette, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectEnroll {
    pub ticket: Option<String>,
}

#[async_trait]
impl ConfigRunner<EnrollCommand> for ProjectEnroll {
    fn len(&self) -> usize {
        match &self.ticket {
            Some(_) => 1,
            None => 0,
        }
    }

    fn into_commands(self) -> Result<Vec<EnrollCommand>> {
        match self.ticket {
            Some(path_or_contents) => Self::get_subcommand(&[path_or_contents]).map(|c| vec![c]),
            None => Ok(vec![]),
        }
    }

    fn get_subcommand(args: &[String]) -> Result<EnrollCommand> {
        if let OckamSubcommand::Project(cmd) = parse_cmd_from_args(EnrollCommand::NAME, args)? {
            if let crate::project::ProjectSubcommand::Enroll(c) = cmd.subcommand {
                return Ok(*c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(EnrollCommand::NAME)
        )))
    }

    async fn pre_run_hooks(
        _ctx: &Context,
        opts: &CommandGlobalOpts,
        _hooks: &PreRunHooks,
        cmd: &mut EnrollCommand,
    ) -> Result<bool> {
        let identity_name = &cmd.cloud_opts.identity;
        let identity = opts
            .state
            .clone()
            .get_named_identity_or_default(identity_name)
            .await?;
        if let Ok(is_enrolled) = opts.state.is_identity_enrolled(identity_name).await {
            if is_enrolled {
                opts.terminal.write_line(&fmt_info!(
                    "Identity {} is already enrolled",
                    color_primary(identity.name())
                ))?;
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::EnrollmentTicket;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn project_enroll_config() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();

        // As contents
        let config = format!("ticket: {enrollment_ticket_hex}");
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].enroll_ticket.as_ref().unwrap(), &enrollment_ticket);

        // As path
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my.ticket");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(enrollment_ticket_hex.as_bytes()).unwrap();
        let config = format!("ticket: {}", file_path.to_str().unwrap());
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].enroll_ticket.as_ref().unwrap(), &enrollment_ticket);
    }
}
