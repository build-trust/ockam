mod nodes;
mod policies;
mod projects;
mod relays;
mod tcp_inlets;
mod tcp_outlets;
mod version;

use nodes::Nodes;
use policies::Policies;
use relays::Relays;
use tcp_inlets::TcpInlets;
use tcp_outlets::TcpOutlets;

use std::collections::BTreeMap;
use std::fmt::Debug;

use clap::{Args, Parser};
use miette::{IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::run::parser::projects::Projects;
use crate::run::parser::version::Version;
use crate::{OckamCommand, OckamSubcommand};

static BINARY_PATH: Lazy<String> = Lazy::new(|| {
    std::env::args()
        .next()
        .expect("Failed to get the binary path")
});

fn binary_path() -> &'static str {
    &BINARY_PATH
}

/// Defines the high-level structure of the configuration file.
///
/// The fields of this struct represents a section of the configuration file. Each section
/// is a list of resources, which, in turn, can be defined in different ways, depending
/// on the nature of the underlying commands.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub version: Version,
    #[serde(flatten)]
    pub projects: Projects,
    #[serde(flatten)]
    pub nodes: Nodes,
    #[serde(flatten)]
    pub policies: Policies,
    #[serde(flatten)]
    pub tcp_outlets: TcpOutlets,
    #[serde(flatten)]
    pub tcp_inlets: TcpInlets,
    #[serde(flatten)]
    pub relays: Relays,
}

impl Config {
    pub fn parse(contents: &str) -> Result<Self> {
        serde_yaml::from_str(contents).into_diagnostic()
    }
}

pub type ResourceName = String;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourcesNamesAndArgs {
    #[serde(flatten)]
    pub items: BTreeMap<ResourceName, KeyValueArgs>,
}

impl ResourcesNamesAndArgs {
    pub fn into_commands<C, F>(self, get_subcommand: F, name_arg: Option<&str>) -> Result<Vec<C>>
    where
        C: Args,
        F: Fn(&[String]) -> Result<C>,
    {
        self.items
            .into_iter()
            .map(|(n, a)| {
                let mut args = match name_arg {
                    None => vec![n],
                    Some(arg) => vec![as_keyword_arg(&arg.to_string()), n],
                };
                args.extend(
                    a.args
                        .into_iter()
                        .flat_map(|(k, v)| vec![as_keyword_arg(&k), resolve(&v)]),
                );
                get_subcommand(&args)
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourcesArgs {
    items: Vec<KeyValueArgs>,
}

impl ResourcesArgs {
    pub fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: Args,
        F: Fn(&[String]) -> Result<C>,
    {
        self.items
            .into_iter()
            .map(|a| {
                let args = a
                    .args
                    .into_iter()
                    .flat_map(|(k, v)| vec![as_keyword_arg(&k), resolve(&v)])
                    .collect::<Vec<_>>();
                get_subcommand(&args)
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourcesNamesWithListOfArgs {
    #[serde(flatten)]
    pub items: BTreeMap<ResourceName, Vec<KeyValueArgs>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyValueArgs {
    #[serde(flatten)]
    pub args: BTreeMap<ArgKey, ArgValue>,
}

pub type ArgKey = String;
pub type ArgValue = String;

pub fn as_keyword_arg(k: &ArgKey) -> String {
    if k.len() == 1 {
        format!("-{k}")
    } else {
        format!("--{k}")
    }
}

/// Resolve environment variables if applicable
pub fn resolve(v: &ArgValue) -> String {
    if v.starts_with('$') {
        let v = v.trim_start_matches('$');
        if let Ok(v) = std::env::var(v) {
            return v;
        }
    }
    v.to_string()
}

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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourcesContainer {
    Name(ResourceName),
    Names(Vec<ResourceName>),
    NamesAndArgs(ResourcesNamesAndArgs),
}

impl ResourcesContainer {
    pub fn into_commands<C, F>(self, get_subcommand: F) -> Result<Vec<C>>
    where
        C: Args,
        F: Fn(&[String]) -> Result<C>,
    {
        match self {
            ResourcesContainer::Name(name) => Ok(vec![get_subcommand(&[name])?]),
            ResourcesContainer::Names(names) => names
                .into_iter()
                .map(|n| get_subcommand(&[n]))
                .collect::<Result<Vec<_>>>(),
            ResourcesContainer::NamesAndArgs(items) => items.into_commands(get_subcommand, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::version::VersionValue;

    #[test]
    fn can_parse_base_config() {
        let config = r#"
            version: '1'

            nodes:
              - n1
              - n2

            policies:
              - at: n1
                resource: r1
                expression: (= subject.component "c1")
              - at: n2
                resource: r2
                expression: (= subject.component "c2")

            tcp-outlets:
              to1:
                to: '6060'
                at: n
              to2:
                to: '6061'

            tcp-inlets:
              ti1:
                from: '6060'
                at: n
              ti2:
                from: '6061'

            relays:
              - r1
              - r2
        "#;
        let parsed: Config = serde_yaml::from_str(config).unwrap();

        let expected = Config {
            version: Version {
                version: VersionValue::latest(),
            },
            projects: Projects { projects: None },
            nodes: Nodes {
                nodes: Some(ResourcesContainer::Names(vec![
                    "n1".to_string(),
                    "n2".to_string(),
                ])),
            },
            policies: Policies {
                policies: Some(ResourcesArgs {
                    items: vec![
                        KeyValueArgs {
                            args: vec![
                                ("at".to_string(), "n1".to_string()),
                                ("resource".to_string(), "r1".to_string()),
                                (
                                    "expression".to_string(),
                                    "(= subject.component \"c1\")".to_string(),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        },
                        KeyValueArgs {
                            args: vec![
                                ("at".to_string(), "n2".to_string()),
                                ("resource".to_string(), "r2".to_string()),
                                (
                                    "expression".to_string(),
                                    "(= subject.component \"c2\")".to_string(),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        },
                    ],
                }),
            },
            tcp_outlets: TcpOutlets {
                tcp_outlets: Some(ResourcesNamesAndArgs {
                    items: vec![
                        (
                            "to1".to_string(),
                            KeyValueArgs {
                                args: vec![
                                    ("to".to_string(), "6060".to_string()),
                                    ("at".to_string(), "n".to_string()),
                                ]
                                .into_iter()
                                .collect(),
                            },
                        ),
                        (
                            "to2".to_string(),
                            KeyValueArgs {
                                args: vec![("to".to_string(), "6061".to_string())]
                                    .into_iter()
                                    .collect(),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                }),
            },
            tcp_inlets: TcpInlets {
                tcp_inlets: Some(ResourcesNamesAndArgs {
                    items: vec![
                        (
                            "ti1".to_string(),
                            KeyValueArgs {
                                args: vec![
                                    ("from".to_string(), "6060".to_string()),
                                    ("at".to_string(), "n".to_string()),
                                ]
                                .into_iter()
                                .collect(),
                            },
                        ),
                        (
                            "ti2".to_string(),
                            KeyValueArgs {
                                args: vec![("from".to_string(), "6061".to_string())]
                                    .into_iter()
                                    .collect(),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                }),
            },
            relays: Relays {
                relays: Some(ResourcesContainer::Names(vec![
                    "r1".to_string(),
                    "r2".to_string(),
                ])),
            },
        };
        assert_eq!(expected, parsed);
    }
}
