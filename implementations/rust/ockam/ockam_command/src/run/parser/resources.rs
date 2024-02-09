use crate::{OckamCommand, OckamSubcommand};
use clap::{Args as ClapArgs, Parser};
use miette::{IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;

static BINARY_PATH: Lazy<String> = Lazy::new(|| {
    std::env::args()
        .next()
        .expect("Failed to get the binary path")
});

fn binary_path() -> &'static str {
    &BINARY_PATH
}

pub type ResourceName = String;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourcesContainer {
    Name(ResourceName),
    List(Vec<ResourceNameOrMap>),
    Map(ResourceNameOrMap),
}

impl ResourcesContainer {
    pub fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        match self {
            ResourcesContainer::Name(name) => Ok(vec![get_subcommand(&[name])?]),
            ResourcesContainer::List(resources) => {
                let mut cmds = vec![];
                for r in resources {
                    cmds.extend(r.into_commands(&get_subcommand, None)?);
                }
                Ok(cmds)
            }
            ResourcesContainer::Map(r) => r.into_commands(get_subcommand, None),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedResources {
    #[serde(flatten)]
    pub items: BTreeMap<ResourceName, Args>,
}

impl NamedResources {
    pub fn into_commands<C, F>(
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
                let mut args = match name_arg_key {
                    None => vec![n],
                    Some(arg) => vec![as_keyword_arg(&arg.to_string()), n],
                };
                args.extend(a.args.into_iter().flat_map(|(k, v)| as_command_arg(k, v)));
                get_subcommand(&args)
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceNameOrMap {
    Name(ResourceName),
    NamedMap(NamedResources),
    RandomlyNamedMap(UnnamedResources),
}

impl ResourceNameOrMap {
    pub fn into_commands<C, F>(
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
                resources.into_commands(get_subcommand, name_arg_key)
            }
            ResourceNameOrMap::RandomlyNamedMap(resources) => {
                resources.into_commands(get_subcommand)
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnnamedResources {
    Single(Args),
    List(Vec<Args>),
}

impl UnnamedResources {
    pub fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: ClapArgs,
        F: Fn(&[String]) -> Result<C>,
    {
        let items = match self {
            UnnamedResources::Single(args) => vec![args],
            UnnamedResources::List(args) => args,
        };
        items
            .into_iter()
            .map(|a| {
                let args = a
                    .args
                    .into_iter()
                    .flat_map(|(k, v)| as_command_arg(k, v))
                    .collect::<Vec<_>>();
                get_subcommand(&args)
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Args {
    #[serde(flatten)]
    pub args: BTreeMap<ArgKey, ArgValue>,
}

pub type ArgKey = String;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    String(String),
    Int(isize),
    Bool(bool),
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

impl Display for ArgValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ArgValue::String(v) => v.to_string(),
            ArgValue::Int(v) => v.to_string(),
            ArgValue::Bool(v) => v.to_string(),
        };
        write!(f, "{}", str)
    }
}

/// Return the command representation of the argument name and its value.
pub fn as_command_arg(k: ArgKey, v: ArgValue) -> Vec<String> {
    match v {
        ArgValue::Bool(v) => {
            // Booleans are passed as a flag, and only when true.
            if v {
                vec![as_keyword_arg(&k)]
            }
            // Otherwise, they are omitted.
            else {
                vec![]
            }
        }
        v => vec![as_keyword_arg(&k), v.to_string()],
    }
}

/// Return the command representation of the argument name
pub fn as_keyword_arg(k: &ArgKey) -> String {
    // If the argument name is a single character, it's the short version of the argument.
    if k.len() == 1 {
        format!("-{k}")
    }
    // Otherwise, it's the long version of the argument.
    else {
        format!("--{k}")
    }
}

/// Return a clap OckamSubcommand instance given the name of the
/// command and the list of arguments
pub fn parse_cmd_from_args(cmd: &str, args: &[String]) -> Result<OckamSubcommand> {
    let args = [binary_path()]
        .into_iter()
        .chain(cmd.split(' '))
        .chain(args.iter().map(|s| s.as_str()))
        .collect::<Vec<_>>();
    Ok(OckamCommand::try_parse_from(args)
        .into_diagnostic()?
        .subcommand)
}

pub trait ArgsToCommands<T> {
    fn into_commands(self) -> Result<Vec<T>>;
}
