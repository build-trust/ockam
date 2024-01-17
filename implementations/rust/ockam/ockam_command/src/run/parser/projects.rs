use crate::project::EnrollCommand;

use crate::run::parser::{
    as_keyword_arg, parse_cmd_from_args, resolve, ResourcesNamesWithListOfArgs,
};
use crate::OckamSubcommand;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Projects {
    pub projects: Option<ResourcesNamesWithListOfArgs>,
}

#[derive(Debug, Default)]
pub struct ProjectCmds {
    pub enroll: Vec<EnrollCommand>,
}

impl Projects {
    pub fn into_commands(self) -> Result<ProjectCmds> {
        let get_enroll_subcommand = |args: &[String]| -> Result<EnrollCommand> {
            if let OckamSubcommand::Project(cmd) = parse_cmd_from_args("project enroll", args)? {
                if let crate::project::ProjectSubcommand::Enroll(c) = cmd.subcommand {
                    return Ok(*c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        let mut cmds = ProjectCmds::default();
        if let Some(c) = self.projects {
            for (name, list_of_args) in c.items.into_iter() {
                match name.as_str() {
                    "enroll" => {
                        for cmd_args in list_of_args {
                            let args = cmd_args
                                .args
                                .into_iter()
                                .flat_map(|(k, v)| vec![as_keyword_arg(&k), resolve(&v)])
                                // Remove "ticket" arg name from args, as it's expected to be a positional argument
                                .filter(|a| a != &as_keyword_arg(&"ticket".to_string()))
                                .collect::<Vec<_>>();
                            let cmd = get_enroll_subcommand(&args)?;
                            cmds.enroll.push(cmd);
                        }
                    }
                    "ticket" => {
                        unimplemented!()
                    }
                    _project_name => {
                        unimplemented!()
                    }
                }
            }
        }
        Ok(cmds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::OneTimeCode;
    use ockam_api::EnrollmentTicket;

    #[test]
    fn tcp_inlet_config() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();
        std::env::set_var("ENROLLMENT_TICKET", enrollment_ticket_hex);
        let config = r#"
            projects:
              enroll:
                - ticket: $ENROLLMENT_TICKET
                  identity: i1
                - ticket: $ENROLLMENT_TICKET
        "#;
        let parsed: Projects = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.enroll.len(), 2);
        assert!(cmds.enroll[0].enroll_ticket.is_some());
        assert_eq!(cmds.enroll[0].cloud_opts.identity.as_ref().unwrap(), "i1");
        assert!(cmds.enroll[1].enroll_ticket.is_some());
    }
}
