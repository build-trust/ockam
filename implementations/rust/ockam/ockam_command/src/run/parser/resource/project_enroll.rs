use miette::{miette, Result};
use ockam_api::colors::color_primary;

use serde::{Deserialize, Serialize};

use crate::project::EnrollCommand;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::Resource;
use crate::{Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectEnroll {
    pub ticket: Option<String>,
}

impl Resource<EnrollCommand> for ProjectEnroll {
    const COMMAND_NAME: &'static str = EnrollCommand::NAME;

    fn args(self) -> Vec<String> {
        if let Some(path_or_contents) = self.ticket {
            vec![path_or_contents]
        } else {
            vec![]
        }
    }
}

impl ProjectEnroll {
    pub fn into_parsed_commands(self) -> Result<Vec<EnrollCommand>> {
        let args = self.args();
        if args.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![Self::get_subcommand(&args)?])
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

    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::cli_state::EnrollmentTicket;

    use super::*;

    #[test]
    fn project_enroll_config() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();

        // As contents
        let config = format!("ticket: {enrollment_ticket_hex}");
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_parsed_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            &enrollment_ticket_hex
        );

        // As path
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my.ticket");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(enrollment_ticket_hex.as_bytes()).unwrap();
        let config = format!("ticket: {}", file_path.to_str().unwrap());
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_parsed_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            file_path.to_str().unwrap()
        );
    }
}
