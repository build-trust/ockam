use serde::{Deserialize, Serialize};

use ockam_node::Context;

use crate::run::parser::config::ConfigParser;
use crate::run::parser::resource::*;
use crate::run::parser::Version;
use crate::CommandGlobalOpts;

/// Defines the high-level structure of the configuration file.
///
/// The fields of this struct represent a section of the configuration file. Each section
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
    pub project_enroll: ProjectEnroll,
    #[serde(flatten)]
    pub nodes: Nodes,
    #[serde(flatten)]
    pub policies: Policies,
    #[serde(flatten)]
    pub tcp_outlets: TcpOutlets,
    #[serde(flatten)]
    pub tcp_inlets: TcpInlets,
    #[serde(flatten)]
    pub kafka_inlet: KafkaInlet,
    #[serde(flatten)]
    pub kafka_outlet: KafkaOutlet,
    #[serde(flatten)]
    pub relays: Relays,
}

impl ConfigParser<'_> for Config {}

impl Config {
    /// Executes the commands described in the configuration to create the desired state.
    ///
    /// More specifically, this struct is responsible for:
    /// - Running the commands in a valid order. For example, nodes will be created before TCP inlets.
    /// - Do the necessary checks to run only the necessary commands. For example, an enrollment ticket won't
    ///  be used if the identity is already enrolled.
    ///
    /// For more details about the parsing, see the [parser](crate::run::parser) module.
    /// You can also check examples of valid configuration files in the demo folder of this module.
    pub async fn run(self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        for cmd in self.parse_commands()? {
            cmd.run(ctx, opts).await?
        }
        Ok(())
    }

    // Build commands and return validation errors
    fn parse_commands(self) -> miette::Result<Vec<ParsedCommands>> {
        Ok(vec![
            self.vaults.into_parsed_commands()?.into(),
            self.identities.into_parsed_commands()?.into(),
            self.project_enroll.into_parsed_commands()?.into(),
            self.nodes.into_parsed_commands()?.into(),
            self.relays.into_parsed_commands(None)?.into(),
            self.policies.into_parsed_commands()?.into(),
            self.tcp_outlets.into_parsed_commands(None)?.into(),
            self.tcp_inlets.into_parsed_commands(None)?.into(),
            self.kafka_inlet.into_parsed_commands(None)?.into(),
            self.kafka_outlet.into_parsed_commands(None)?.into(),
        ])
    }

    pub async fn parse_and_run(
        ctx: &Context,
        opts: CommandGlobalOpts,
        contents: &str,
    ) -> miette::Result<()> {
        let config = Config::parse(&Config::resolve(contents)?)?;
        config.run(ctx, &opts).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::run::parser::building_blocks::*;
    use crate::run::parser::VersionValue;

    use super::*;

    #[test]
    fn parse_complete_config() {
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

            kafka-inlet:
                from: 9092
                at: n
                to: /project/project_name
                port-range: 1000-2000

            kafka-outlet:
                bootstrap-server: 192.168.1.1:9092
                at: n

            relays:
              - r1
              - r2
        "#;
        let parsed = Config::parse(config).unwrap();

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
            project_enroll: ProjectEnroll {
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
            kafka_inlet: KafkaInlet {
                kafka_inlet: Some(UnnamedResources::Single(Args {
                    args: vec![
                        ("from".to_string(), "9092".into()),
                        ("at".to_string(), "n".into()),
                        ("to".to_string(), "/project/project_name".into()),
                        ("port-range".to_string(), "1000-2000".into()),
                    ]
                    .into_iter()
                    .collect(),
                })),
            },
            kafka_outlet: KafkaOutlet {
                kafka_outlet: Some(UnnamedResources::Single(Args {
                    args: vec![
                        ("bootstrap-server".to_string(), "192.168.1.1:9092".into()),
                        ("at".to_string(), "n".into()),
                    ]
                    .into_iter()
                    .collect(),
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

    #[test]
    fn resolve_variables() {
        std::env::set_var("SUFFIX", "node");
        let config = r#"
            variables:
              prefix: ockam
              ticket_path: ./path/to/ticket

            ticket: ${ticket_path}

            nodes:
              - ${prefix}_n1_${SUFFIX}
              - ${prefix}_n2_${SUFFIX}
        "#;
        let resolved = Config::resolve(config).unwrap();
        let parsed = Config::parse(&resolved).unwrap();
        let expected = Config {
            version: Version {
                version: VersionValue::latest(),
            },
            vaults: Vaults { vaults: None },
            identities: Identities { identities: None },
            project_enroll: ProjectEnroll {
                ticket: Some("./path/to/ticket".to_string()),
            },
            nodes: Nodes {
                nodes: Some(ResourcesContainer::List(vec![
                    ResourceNameOrMap::Name("ockam_n1_node".to_string()),
                    ResourceNameOrMap::Name("ockam_n2_node".to_string()),
                ])),
            },
            policies: Policies { policies: None },
            tcp_outlets: TcpOutlets { tcp_outlets: None },
            tcp_inlets: TcpInlets { tcp_inlets: None },
            kafka_inlet: KafkaInlet { kafka_inlet: None },
            kafka_outlet: KafkaOutlet { kafka_outlet: None },
            relays: Relays { relays: None },
        };
        assert_eq!(expected, parsed);
    }

    #[test]
    fn parse_demo_config_files() {
        let files = std::fs::read_dir(demo_config_files_dir()).unwrap();
        for file in files {
            let file = file.unwrap();
            let path = file.path();
            let contents = std::fs::read_to_string(&path).unwrap();
            let res = Config::parse(&contents);
            res.unwrap();
        }
    }

    #[test]
    fn parse_demo_config_file_1() {
        let path = demo_config_files_dir().join("1.portal.single-machine.yaml");
        let config = Config::resolve(&std::fs::read_to_string(path).unwrap()).unwrap();
        let parsed = Config::parse(&config).unwrap();
        assert_eq!(parsed.version.version, VersionValue::latest());
        assert_eq!(parsed.vaults.vaults, None);
        assert_eq!(parsed.identities.identities, None);
        assert_eq!(parsed.project_enroll.ticket, None);
        assert_eq!(parsed.nodes.nodes, None);
        assert_eq!(parsed.policies.policies, None);
        assert_eq!(
            parsed.tcp_outlets.tcp_outlets,
            Some(ResourceNameOrMap::NamedMap(NamedResources {
                items: vec![(
                    "db-outlet".to_string(),
                    Args {
                        args: vec![("to".to_string(), "5432".into())]
                            .into_iter()
                            .collect(),
                    },
                ),]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
            }))
        );
        assert_eq!(
            parsed.tcp_inlets.tcp_inlets,
            Some(ResourceNameOrMap::NamedMap(NamedResources {
                items: vec![(
                    "web-inlet".to_string(),
                    Args {
                        args: vec![("from".to_string(), "4000".into())]
                            .into_iter()
                            .collect(),
                    },
                ),]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
            }))
        );
        assert_eq!(
            parsed.relays.relays,
            Some(ResourcesContainer::NameOrMap(ResourceNameOrMap::Name(
                "default".to_string()
            )))
        );
    }

    #[test]
    fn parse_demo_config_file_2_inlet() {
        let path = demo_config_files_dir().join("2.portal.inlet.yaml");
        let config = Config::resolve(&std::fs::read_to_string(path).unwrap()).unwrap();
        let parsed = Config::parse(&config).unwrap();
        assert_eq!(parsed.version.version, VersionValue::latest());
        assert_eq!(parsed.vaults.vaults, None);
        assert_eq!(parsed.identities.identities, None);
        assert_eq!(
            parsed.project_enroll.ticket,
            Some("webapp.ticket".to_string())
        );
        assert_eq!(
            parsed.nodes.nodes,
            Some(ResourcesContainer::NameOrMap(ResourceNameOrMap::Name(
                "web".to_string()
            )))
        );
        assert_eq!(parsed.policies.policies, None);
        assert_eq!(parsed.tcp_outlets.tcp_outlets, None);
        assert_eq!(
            parsed.tcp_inlets.tcp_inlets,
            Some(ResourceNameOrMap::NamedMap(NamedResources {
                items: vec![(
                    "web-inlet".to_string(),
                    Args {
                        args: vec![
                            ("from".to_string(), "4000".into()),
                            ("via".to_string(), "db".into()),
                            ("allow".to_string(), "component.db".into()),
                        ]
                        .into_iter()
                        .collect(),
                    },
                ),]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
            }))
        );
        assert_eq!(parsed.relays.relays, None);
    }

    #[test]
    fn parse_demo_config_file_2_outlet() {
        let path = demo_config_files_dir().join("2.portal.outlet.yaml");
        let config = Config::resolve(&std::fs::read_to_string(path).unwrap()).unwrap();
        let parsed = Config::parse(&config).unwrap();
        assert_eq!(parsed.version.version, VersionValue::latest());
        assert_eq!(parsed.vaults.vaults, None);
        assert_eq!(parsed.identities.identities, None);
        assert_eq!(parsed.project_enroll.ticket, Some("db.ticket".to_string()));
        assert_eq!(
            parsed.nodes.nodes,
            Some(ResourcesContainer::NameOrMap(ResourceNameOrMap::Name(
                "db".to_string()
            )))
        );
        assert_eq!(parsed.policies.policies, None);
        assert_eq!(
            parsed.tcp_outlets.tcp_outlets,
            Some(ResourceNameOrMap::NamedMap(NamedResources {
                items: vec![(
                    "db-outlet".to_string(),
                    Args {
                        args: vec![
                            ("to".to_string(), "5432".into()),
                            ("allow".to_string(), "component.web".into()),
                        ]
                        .into_iter()
                        .collect(),
                    },
                ),]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
            }))
        );
        assert_eq!(parsed.tcp_inlets.tcp_inlets, None);
        assert_eq!(
            parsed.relays.relays,
            Some(ResourcesContainer::NameOrMap(ResourceNameOrMap::Name(
                "db".to_string()
            )))
        );
    }

    fn demo_config_files_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("src")
            .join("run")
            .join("demo_config_files")
    }
}
