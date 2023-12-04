use crate::project::{CreateCommand, EnrollCommand};
use std::collections::BTreeMap;

use crate::run::parser::resources::{
    as_command_arg, as_keyword_arg, parse_cmd_from_args, ResourceName, UnnamedResources,
};
use crate::OckamSubcommand;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Projects {
    pub projects: Option<BTreeMap<ResourceName, UnnamedResources>>,
}

#[derive(Debug, Default)]
pub struct ProjectCmds {
    pub create: Vec<CreateCommand>,
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
        let get_create_subcommand = |args: &[String]| -> Result<CreateCommand> {
            if let OckamSubcommand::Project(cmd) = parse_cmd_from_args("project create", args)? {
                if let crate::project::ProjectSubcommand::Create(c) = cmd.subcommand {
                    return Ok(c);
                }
            }
            Err(miette!("Failed to parse command"))
        };
        let mut cmds = ProjectCmds::default();
        if let Some(c) = self.projects {
            for (name, args) in c.into_iter() {
                let mut list_of_args = match args {
                    UnnamedResources::Single(args) => vec![args],
                    UnnamedResources::List(args) => args,
                };
                match name.as_str() {
                    "enroll" => {
                        for cmd_args in list_of_args {
                            let args = cmd_args
                                .args
                                .into_iter()
                                .flat_map(|(k, v)| as_command_arg(k, v))
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
                    project_name => {
                        let args = list_of_args.pop().expect("There should be one element");
                        let mut args = args
                            .args
                            .into_iter()
                            .flat_map(|(k, v)| as_command_arg(k, v))
                            // Remove "space" arg name from args, as it's expected to be a positional argument
                            .filter(|a| a != &as_keyword_arg(&"space".to_string()))
                            .collect::<Vec<_>>();
                        // Add the project name at the end, as the first positional argument must be the space name
                        args.push(project_name.to_string());
                        let cmd = get_create_subcommand(&args)?;
                        cmds.create.push(cmd);
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
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::EnrollmentTicket;

    #[test]
    fn project_enroll_config() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();
        std::env::set_var("ENROLLMENT_TICKET", enrollment_ticket_hex);

        // Single
        let config = r#"
            projects:
              enroll:
                ticket: $ENROLLMENT_TICKET
        "#;
        let parsed: Projects = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();
        assert_eq!(cmds.enroll.len(), 1);
        assert!(cmds.enroll[0].enroll_ticket.is_some());

        // List
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

    #[test]
    fn projects_full_config() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();
        std::env::set_var("ENROLLMENT_TICKET", enrollment_ticket_hex);
        let config = r#"
            projects:
              p1:
                space: s1
                identity: i1
              p2:
                space: s2
              enroll:
                - ticket: $ENROLLMENT_TICKET
                  identity: i1
                - ticket: $ENROLLMENT_TICKET
        "#;
        let parsed: Projects = serde_yaml::from_str(config).unwrap();
        let cmds = parsed.into_commands().unwrap();

        assert_eq!(cmds.create.len(), 2);
        assert_eq!(&cmds.create[0].project_name, "p1");
        assert_eq!(&cmds.create[0].space_name, "s1");
        assert_eq!(cmds.create[0].cloud_opts.identity.as_ref().unwrap(), "i1");
        assert_eq!(&cmds.create[1].project_name, "p2");
        assert_eq!(&cmds.create[1].space_name, "s2");
        assert!(cmds.create[1].cloud_opts.identity.is_none());

        assert_eq!(cmds.enroll.len(), 2);
        assert!(cmds.enroll[0].enroll_ticket.is_some());
        assert_eq!(cmds.enroll[0].cloud_opts.identity.as_ref().unwrap(), "i1");
        assert!(cmds.enroll[1].enroll_ticket.is_some());
    }
}
