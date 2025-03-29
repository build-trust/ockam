use miette::{miette, Result};
use ockam_api::colors::color_primary;

use serde::{Deserialize, Serialize};

use crate::project::EnrollCommand;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::Resource;
use crate::{Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProjectEnroll {
    pub ticket: Option<String>,
}

impl Resource<EnrollCommand> for ProjectEnroll {
    const COMMAND_NAME: &'static str = EnrollCommand::NAME;

    fn args(self) -> Vec<String> {
        if let Some(ticket) = self.ticket {
            vec![ticket]
        } else {
            vec![]
        }
    }
}

impl ProjectEnroll {
    pub fn into_parsed_commands(
        self,
        default_identity_name: Option<&String>,
    ) -> Result<Vec<EnrollCommand>> {
        let args = self.args();
        if args.is_empty() {
            Ok(vec![])
        } else {
            let mut cmd = Self::get_subcommand(&args)?;
            if cmd.identity_opts.identity_name.is_none() {
                cmd.identity_opts.identity_name = default_identity_name.cloned();
            }
            Ok(vec![cmd])
        }
    }

    fn get_subcommand(args: &[String]) -> Result<EnrollCommand> {
        if let OckamSubcommand::Project(cmd) = parse_cmd_from_args(EnrollCommand::NAME, args)? {
            if let crate::project::ProjectSubcommand::Enroll(c) = cmd.subcommand {
                return Ok(c);
            }
        }
        Err(miette!(format!(
            "Failed to parse {} command",
            color_primary(EnrollCommand::NAME)
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::tempdir;

    use ockam_api::cli_state::ExportedEnrollmentTicket;

    use super::*;

    #[test]
    fn project_enroll_config() {
        let enrollment_ticket = ExportedEnrollmentTicket::new_test();
        let enrollment_ticket_encoded = enrollment_ticket.to_string();

        // As contents
        let config = format!("ticket: {enrollment_ticket_encoded}");
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_parsed_commands(None).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            &enrollment_ticket_encoded
        );

        // As path
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my.ticket");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(enrollment_ticket_encoded.as_bytes())
            .unwrap();
        let config = format!("ticket: {}", file_path.to_str().unwrap());
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed
            .into_parsed_commands(Some(&"identity-name".to_string()))
            .unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            file_path.to_str().unwrap()
        );
        assert_eq!(
            cmds[0].identity_opts.identity_name.as_deref(),
            Some("identity-name")
        );
    }
}
