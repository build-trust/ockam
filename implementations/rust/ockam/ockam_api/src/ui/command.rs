use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug)]
pub struct Commands {
    pub(crate) commands: Vec<Command>,
}

#[derive(Clone)]
pub(crate) struct Command {
    pub(crate) name: &'static str,
    pub(crate) custom_name: &'static str,
}

impl Debug for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("name", &self.name)
            .field("custom_name", &self.custom_name)
            .finish()
    }
}

impl Commands {
    pub fn new(commands: &'static str) -> Self {
        let commands = commands
            .split(',')
            .filter_map(|c| {
                if c.is_empty() {
                    return None;
                }
                let mut parts = c.split('=');
                let name = match parts.next() {
                    Some(name) => name,
                    None => return None,
                };
                let custom_name = parts.next().unwrap_or(name);
                Some(Command { name, custom_name })
            })
            .collect();
        Self { commands }
    }

    pub fn hide(&self, command_name: &'static str) -> bool {
        // No restrictions
        if self.commands.is_empty() {
            return false;
        }
        // Check if the command is in the list of hidden commands
        !self.commands.iter().any(|c| c.name == command_name)
    }

    pub fn name(&self, command_name: &'static str) -> &'static str {
        // No restrictions
        if self.commands.is_empty() {
            return command_name;
        }
        // Check the custom name in the list of renamed commands
        self.commands
            .iter()
            .find(|c| c.name == command_name)
            .map_or(command_name, |c| c.custom_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hide() {
        let commands = Commands::new("node=host,project,enroll");
        assert!(!commands.hide("node"));
        assert!(!commands.hide("project"));
        assert!(!commands.hide("enroll"));
        assert!(commands.hide("command4"));

        let commands = Commands::new("");
        assert!(!commands.hide("command1"));
    }

    #[test]
    fn test_commands() {
        let commands = Commands::new("node=host,project,enroll");
        assert_eq!(commands.name("node"), "host");
        assert_eq!(commands.name("project"), "project");
        assert_eq!(commands.name("enroll"), "enroll");
        assert_eq!(commands.name("command4"), "command4");

        let commands = Commands::new("");
        assert_eq!(commands.name("command1"), "command1");
    }
}
