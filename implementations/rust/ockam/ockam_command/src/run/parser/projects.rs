use crate::project::EnrollCommand;

use crate::run::parser::resources::{parse_cmd_from_args, resolve, ArgValue};
use crate::run::parser::ArgsToCommands;
use crate::OckamSubcommand;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Projects {
    pub ticket: Option<String>,
}

impl ArgsToCommands<EnrollCommand> for Projects {
    fn into_commands(self) -> Result<Vec<EnrollCommand>> {
        let get_subcommand = |args: &[String]| -> Result<EnrollCommand> {
            if let OckamSubcommand::Project(cmd) = parse_cmd_from_args("project enroll", args)? {
                if let crate::project::ProjectSubcommand::Enroll(c) = cmd.subcommand {
                    return Ok(*c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        match self.ticket {
            Some(path_or_contents) => {
                get_subcommand(&[resolve(&ArgValue::String(path_or_contents))]).map(|c| vec![c])
            }
            None => Ok(vec![]),
        }
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
        std::env::set_var("ENROLLMENT_TICKET", &enrollment_ticket_hex);
        let config = "ticket: $ENROLLMENT_TICKET";
        let parsed: Projects = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].enroll_ticket.as_ref().unwrap(), &enrollment_ticket);

        // As path
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("my.ticket");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(enrollment_ticket_hex.as_bytes()).unwrap();
        let config = format!("ticket: {}", file_path.to_str().unwrap());
        let parsed: Projects = serde_yaml::from_str(&config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].enroll_ticket.as_ref().unwrap(), &enrollment_ticket);
    }
}
