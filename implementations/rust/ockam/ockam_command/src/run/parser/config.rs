use crate::run::parser::identities::Identities;
use crate::run::parser::nodes::Nodes;
use crate::run::parser::policies::Policies;
use crate::run::parser::projects::Projects;
use crate::run::parser::relays::Relays;
use crate::run::parser::tcp_inlets::TcpInlets;
use crate::run::parser::tcp_outlets::TcpOutlets;
use crate::run::parser::vaults::Vaults;
use crate::run::parser::version::Version;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

/// Defines the high-level structure of the configuration file.
///
/// The fields of this struct represents a section of the configuration file. Each section
/// is a list of resources, which, in turn, can be defined in different ways, depending
/// on the nature of the underlying commands.
///
/// Each resource can be configured using the arguments available for the corresponding command.
/// For example, the `node` resource accepts any arguments that the `node create` command accepts.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub version: Version,
    #[serde(flatten)]
    pub vaults: Vaults,
    #[serde(flatten)]
    pub identities: Identities,
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
    pub fn parse(contents: &str) -> miette::Result<Self> {
        serde_yaml::from_str(contents).into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::resources::*;
    use crate::run::parser::version::VersionValue;
    use std::collections::BTreeMap;

    #[test]
    fn can_parse_base_config() {
        let config = r#"
            vaults:
              - v1
              - v2

            identities:
              - i1
              - i2:
                  vault: v2

            ticket: ./path/to/ticket

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
                to: 6060
                at: n
              to2:
                to: 6061

            tcp-inlets:
              ti1:
                from: 6060
                at: n
              ti2:
                from: 6061

            relays:
              - r1
              - r2
        "#;
        let parsed: Config = serde_yaml::from_str(config).unwrap();

        let expected = Config {
            version: Version {
                version: VersionValue::latest(),
            },
            vaults: Vaults {
                vaults: Some(ResourcesContainer::List(vec![
                    ResourceNameOrMap::Name("v1".to_string()),
                    ResourceNameOrMap::Name("v2".to_string()),
                ])),
            },
            identities: Identities {
                identities: Some(ResourcesContainer::List(vec![
                    ResourceNameOrMap::Name("i1".to_string()),
                    ResourceNameOrMap::NamedMap(NamedResources {
                        items: vec![(
                            "i2".to_string(),
                            Args {
                                args: vec![("vault".to_string(), "v2".into())]
                                    .into_iter()
                                    .collect(),
                            },
                        )]
                        .into_iter()
                        .collect::<BTreeMap<_, _>>(),
                    }),
                ])),
            },
            projects: Projects {
                ticket: Some("./path/to/ticket".to_string()),
            },
            nodes: Nodes {
                nodes: Some(ResourcesContainer::List(vec![
                    ResourceNameOrMap::Name("n1".to_string()),
                    ResourceNameOrMap::Name("n2".to_string()),
                ])),
            },
            policies: Policies {
                policies: Some(UnnamedResources::List(vec![
                    Args {
                        args: vec![
                            ("at".to_string(), "n1".into()),
                            ("resource".to_string(), "r1".into()),
                            (
                                "expression".to_string(),
                                "(= subject.component \"c1\")".into(),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    Args {
                        args: vec![
                            ("at".to_string(), "n2".into()),
                            ("resource".to_string(), "r2".into()),
                            (
                                "expression".to_string(),
                                "(= subject.component \"c2\")".into(),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                ])),
            },
            tcp_outlets: TcpOutlets {
                tcp_outlets: Some(ResourceNameOrMap::NamedMap(NamedResources {
                    items: vec![
                        (
                            "to1".to_string(),
                            Args {
                                args: vec![
                                    ("to".to_string(), "6060".into()),
                                    ("at".to_string(), "n".into()),
                                ]
                                .into_iter()
                                .collect(),
                            },
                        ),
                        (
                            "to2".to_string(),
                            Args {
                                args: vec![("to".to_string(), "6061".into())]
                                    .into_iter()
                                    .collect(),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                })),
            },
            tcp_inlets: TcpInlets {
                tcp_inlets: Some(ResourceNameOrMap::NamedMap(NamedResources {
                    items: vec![
                        (
                            "ti1".to_string(),
                            Args {
                                args: vec![
                                    ("from".to_string(), "6060".into()),
                                    ("at".to_string(), "n".into()),
                                ]
                                .into_iter()
                                .collect(),
                            },
                        ),
                        (
                            "ti2".to_string(),
                            Args {
                                args: vec![("from".to_string(), "6061".into())]
                                    .into_iter()
                                    .collect(),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
                })),
            },
            relays: Relays {
                relays: Some(ResourcesContainer::List(vec![
                    ResourceNameOrMap::Name("r1".to_string()),
                    ResourceNameOrMap::Name("r2".to_string()),
                ])),
            },
        };
        assert_eq!(expected, parsed);
    }
}
