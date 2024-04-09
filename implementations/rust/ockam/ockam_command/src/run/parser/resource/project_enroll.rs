use async_trait::async_trait;
use miette::{miette, Result};
use ockam_api::colors::color_primary;
use serde::{Deserialize, Serialize};

use crate::project::EnrollCommand;
use crate::run::parser::resource::traits::CommandsParser;
use crate::run::parser::resource::utils::parse_cmd_from_args;
use crate::run::parser::resource::ValuesOverrides;
use crate::{Command, OckamSubcommand};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectEnroll {
    pub ticket: Option<String>,
}

impl ProjectEnroll {
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
}

#[async_trait]
impl CommandsParser<EnrollCommand> for ProjectEnroll {
    fn parse_commands(self, _overrides: &ValuesOverrides) -> Result<Vec<EnrollCommand>> {
        match self.ticket {
            Some(path_or_contents) => Ok(vec![Self::get_subcommand(&[path_or_contents])?]),
            None => Ok(vec![]),
        }
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
        let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            &enrollment_ticket
        );

        // As path
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my.ticket");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(enrollment_ticket_hex.as_bytes()).unwrap();
        let config = format!("ticket: {}", file_path.to_str().unwrap());
        let parsed: ProjectEnroll = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.parse_commands(&ValuesOverrides::default()).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds[0].enrollment_ticket.as_ref().unwrap(),
            &enrollment_ticket
        );
    }
}
