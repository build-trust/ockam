use clap::Args as ClapArgs;
use miette::{miette, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;

/// This trait defines the methods used to convert a set of arguments that describe a section of
/// the configuration file into a list of commands of the same kind.
///
/// For example, the following section would describe a list of nodes in a yaml configuration file:
/// ```yaml
/// nodes:
///  n1:
///   identity: ...
///   tcp-listener-address: ...
///  n2:
///   identity: ...
///   tcp-listener-address: ...
/// ```
/// Now, a struct implementing this trait will be able to convert the above section into a list of
/// Clap command instances that can be executed to create those nodes.
pub trait ArgsToCommands: Sized {
    /// Given a function that can convert a set of arguments into a command, this method will
    /// return all the commands that can be created from a section of the configuration file.
    fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        self.into_commands_with_name_arg(get_subcommand, None)
    }

    /// Similar to [`into_commands`](Self::into_commands), but passing the name of the argument
    /// in the configuration file that will be used as the name of the resource in relative the command.
    fn into_commands_with_name_arg<C, F>(
        self,
        _get_subcommand: F,
        _name_arg_key: Option<&str>,
    ) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        Err(miette!("The command does not support named resources"))
    }

    /// Returns the number of commands that can be created from the section of the configuration file.
    fn len(&self) -> usize;
}

/// A resource identified only by its name.
///
/// E.g. `vaults: v1`
pub type ResourceName = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourcesContainer {
    NameOrMap(ResourceNameOrMap),
    List(Vec<ResourceNameOrMap>),
}

impl ArgsToCommands for ResourcesContainer {
    fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        match self {
            ResourcesContainer::NameOrMap(r) => r.into_commands(get_subcommand),
            ResourcesContainer::List(resources) => {
                let mut cmds = vec![];
                for r in resources {
                    cmds.extend(r.into_commands(&get_subcommand)?);
                }
                Ok(cmds)
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            ResourcesContainer::NameOrMap(r) => r.len(),
            ResourcesContainer::List(resources) => resources.iter().map(|r| r.len()).sum(),
        }
    }
}

/// A list of resources identified by their name and a set of arguments.
///
/// E.g.
/// ```yaml
/// vaults:
///   v1:
///     path: "./v1.path"
///     aws-kms: false
///   v2:
///     path: "./v2.path"
///     aws-kms: true
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedResources {
    #[serde(flatten)]
    pub items: BTreeMap<ResourceName, Args>,
}

impl ArgsToCommands for NamedResources {
    fn into_commands_with_name_arg<C, F>(
        self,
        get_subcommand: F,
        name_arg_key: Option<&str>,
    ) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        self.items
            .into_iter()
            .map(|(n, a)| {
                // Add the name of the resource as the first argument
                let mut parsed_args = match name_arg_key {
                    None => vec![n],
                    Some(arg) => {
                        // Use the given argument key as the name of the resource
                        let arg = arg.into();
                        let value = a
                            .args
                            .get(&arg)
                            .cloned()
                            .unwrap_or(ArgValue::String(n.to_string()));

                        as_command_arg(arg, value)
                    }
                };
                // Remove the name of the resource from the arguments
                let args = if let Some(arg) = name_arg_key {
                    a.args
                        .into_iter()
                        .filter(|(k, _)| k.as_str() != arg)
                        .collect::<BTreeMap<_, _>>()
                } else {
                    a.args
                };
                // Add the rest of the arguments
                parsed_args.extend(as_command_args(args));
                get_subcommand(&parsed_args)
            })
            .collect::<Result<Vec<_>>>()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceNameOrMap {
    Name(ResourceName),
    NamedMap(NamedResources),
    RandomlyNamedMap(UnnamedResources),
}

impl ArgsToCommands for ResourceNameOrMap {
    fn into_commands_with_name_arg<C, F>(
        self,
        get_subcommand: F,
        name_arg_key: Option<&str>,
    ) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        match self {
            ResourceNameOrMap::Name(name) => Ok(vec![get_subcommand(&[name])?]),
            ResourceNameOrMap::NamedMap(resources) => {
                resources.into_commands_with_name_arg(get_subcommand, name_arg_key)
            }
            ResourceNameOrMap::RandomlyNamedMap(resources) => {
                resources.into_commands(get_subcommand)
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            ResourceNameOrMap::Name(_) => 1,
            ResourceNameOrMap::NamedMap(r) => r.len(),
            ResourceNameOrMap::RandomlyNamedMap(r) => r.len(),
        }
    }
}

/// A list of resources identified by a set of arguments, without a name.
///
/// E.g.
/// ```yaml
/// vaults:
///   - path: "./v1.path"
///     aws-kms: false
///   - path: "./v2.path"
///     aws-kms: false
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnnamedResources {
    Single(Args),
    List(Vec<Args>),
}

impl ArgsToCommands for UnnamedResources {
    fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        let items = match self {
            UnnamedResources::Single(args) => vec![args],
            UnnamedResources::List(items) => items,
        };
        items
            .into_iter()
            .map(|a| get_subcommand(&as_command_args(a.args)))
            .collect::<Result<Vec<_>>>()
    }

    fn len(&self) -> usize {
        match self {
            UnnamedResources::Single(_) => 1,
            UnnamedResources::List(items) => items.len(),
        }
    }
}

/// A set of key/value pairs for a given indentation level.
///
/// E.g.
/// ```yaml
/// path: "./v1.path"
/// aws-kms: false
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    pub args: BTreeMap<ArgKey, ArgValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Eq, Hash)]
#[serde(transparent)]
pub struct ArgKey(String);

impl ArgKey {
    pub fn new<S: Into<String>>(s: S) -> Self {
        ArgKey(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ArgKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ArgKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ArgKey {
    fn from(s: &str) -> Self {
        ArgKey(s.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    String(String),
    Int(isize),
    Bool(bool),
    List(Vec<ArgValue>),
}

impl From<&str> for ArgValue {
    fn from(s: &str) -> Self {
        if let Ok(v) = s.parse::<isize>() {
            return ArgValue::Int(v);
        }
        if let Ok(v) = s.parse::<bool>() {
            return ArgValue::Bool(v);
        }
        ArgValue::String(s.to_string())
    }
}

impl From<String> for ArgValue {
    fn from(s: String) -> Self {
        if let Ok(v) = s.parse::<isize>() {
            return ArgValue::Int(v);
        }
        if let Ok(v) = s.parse::<bool>() {
            return ArgValue::Bool(v);
        }
        ArgValue::String(s)
    }
}

impl From<bool> for ArgValue {
    fn from(b: bool) -> Self {
        ArgValue::Bool(b)
    }
}

impl From<isize> for ArgValue {
    fn from(i: isize) -> Self {
        ArgValue::Int(i)
    }
}

/// Returns the command representation of a set of arguments
pub fn as_command_args(args: BTreeMap<ArgKey, ArgValue>) -> Vec<String> {
    args.into_iter()
        .flat_map(|(k, v)| as_command_arg(k, v))
        .collect::<Vec<_>>()
}

/// Return the command representation of the argument name and its value.
fn as_command_arg(key: ArgKey, value: ArgValue) -> Vec<String> {
    match value {
        ArgValue::List(values) => values
            .into_iter()
            .flat_map(|value| as_command_arg(key.clone(), value))
            .collect(),
        ArgValue::Bool(value) => {
            // Booleans are passed as a flag, and only when true.
            if value {
                vec![as_keyword_arg(&key)]
            }
            // Otherwise, they are omitted.
            else {
                vec![]
            }
        }
        ArgValue::Int(value) => vec![as_keyword_arg(&key), value.to_string()],
        ArgValue::String(value) => vec![as_keyword_arg(&key), value],
    }
}

/// Return the command representation of the argument name
fn as_keyword_arg(k: &ArgKey) -> String {
    // If the argument name is a single character, it's the short version of the argument.
    if k.as_str().len() == 1 {
        format!("-{k}")
    }
    // Otherwise, it's the long version of the argument.
    else {
        format!("--{k}")
    }
}
