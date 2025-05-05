use miette::{IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfig {
    #[serde(alias = "zone_name")]
    pub name: String,
    pub pods: Vec<Pod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pod {
    pub name: String,
    pub containers: Vec<Container>,
    #[serde(default, alias = "portal")]
    pub portals: Portals,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Portals {
    #[serde(default, alias = "inlet", alias = "tcp-inlets", alias = "tcp-inlet")]
    pub inlets: Vec<Inlet>,
    #[serde(
        default,
        alias = "outlet",
        alias = "tcp-outlets",
        alias = "tcp-outlets"
    )]
    pub outlets: Vec<Outlet>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inlet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub from: String,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outlet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub to: String,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub name: String,
    pub image: String,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

impl ZoneConfig {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, miette::Error> {
        let content = std::fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err(format!(
                "Failed to read zone config file at {}",
                path.as_ref().display()
            ))?;
        let self_ = if content.starts_with("{") {
            serde_json::from_str::<Self>(&content)
                .map_err(|e| miette::miette!(format!("Failed to parse JSON zone config: {}", e)))
        } else {
            serde_yaml::from_str::<Self>(&content)
                .map_err(|e| miette::miette!(format!("Failed to parse YAML zone config: {}", e)))
        }?;
        self_.validate()?;
        Ok(self_)
    }

    fn validate(&self) -> Result<(), miette::Error> {
        // Limit zone name to 10 chars
        if self.name.len() > 10 {
            return Err(miette::miette!("Zone name exceeds 10 characters"));
        }

        // Limit pod name to 10 chars
        for pod in &self.pods {
            if pod.name.len() > 10 {
                return Err(miette::miette!(format!(
                    "Pod name '{}' exceeds 10 characters",
                    pod.name
                )));
            }
        }

        // Limit container name to 10 chars
        for pod in &self.pods {
            for container in &pod.containers {
                if container.name.len() > 10 {
                    return Err(miette::miette!(format!(
                        "Container name '{}' exceeds 10 characters",
                        container.name
                    )));
                }
            }
        }

        // Check for duplicate pod names or container names
        let mut pod_names = HashMap::new();
        for pod in &self.pods {
            if let Some(existing) = pod_names.insert(&pod.name, pod) {
                return Err(miette::miette!(format!(
                    "Duplicate pod name '{}' found in zone configuration",
                    existing.name
                )));
            }
            let mut container_names = HashMap::new();
            for container in &pod.containers {
                if let Some(existing) = container_names.insert(&container.name, container) {
                    return Err(miette::miette!(format!(
                        "Duplicate container name '{}' found in pod '{}'",
                        existing.name, pod.name
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn get_local_images_names(&self) -> Vec<String> {
        let is_local = |image_name: &str| {
            // Assumes remote images follow the format "[registry/][username/]name[:tag]"
            // while local images are just "name[:tag]".
            !image_name.contains('/')
        };
        self.pods
            .iter()
            .flat_map(|pod| pod.containers.iter())
            .filter_map(|container| {
                let image_name = container.image.trim();
                if image_name.is_empty() {
                    None
                } else if is_local(image_name) {
                    Some(image_name.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn replace_image_name(
        &mut self,
        image_name: &str,
        ecr_repo_uri: &str,
    ) -> Result<(), miette::Error> {
        let mut replaced = false;

        // Extract the base name without tag
        let base_image_name = image_name.split(':').next().unwrap_or(image_name);

        for pod in &mut self.pods {
            for container in &mut pod.containers {
                // Extract container image base without tag
                let container_base = container
                    .image
                    .split(':')
                    .next()
                    .unwrap_or(&container.image);

                if container_base == base_image_name {
                    // If the container image has a tag, preserve it
                    if let Some((_, tag)) = container.image.split_once(':') {
                        container.image = format!("{}:{}", ecr_repo_uri, tag);
                    } else {
                        container.image = ecr_repo_uri.to_string();
                    }
                    replaced = true;
                }
            }
        }

        if !replaced {
            return Err(miette::miette!(
                "Image '{}' was not found in the zone configuration",
                image_name
            ));
        }

        Ok(())
    }

    /// Returns the name of the "main" pod, which is either:
    /// - The only pod if there's just one pod
    /// - The pod named "main" if it exists
    /// - Error otherwise
    pub fn get_main_pod(&self) -> miette::Result<&Pod> {
        if self.pods.is_empty() {
            return Err(miette::miette!("No pods defined in zone configuration"));
        }
        if self.pods.len() == 1 {
            // If there's only one pod, return it
            Ok(&self.pods[0])
        } else {
            // Try to find a pod named "main"
            self.pods
                .iter()
                .find(|pod| pod.name == "main")
                .ok_or_else(|| miette::miette!("Multiple pods defined, but none is named 'main'"))
        }
    }
}

impl Pod {
    pub fn get_outlets(&self) -> PodOutlets {
        // Collect all outlets
        let mut repl = None;
        let mut rest = Vec::new();
        for outlet in &self.portals.outlets {
            if outlet.name.as_deref() == Some("repl") {
                repl = Some(outlet.clone());
            } else {
                rest.push(outlet.clone());
            }
        }

        // If there is only one outlet, return it as the repl
        if repl.is_none() && rest.len() == 1 {
            repl = Some(rest.remove(0));
        }

        PodOutlets { repl, rest }
    }
}

pub struct PodOutlets {
    pub repl: Option<Outlet>,
    pub rest: Vec<Outlet>,
}

impl Outlet {
    pub fn get_port(&self) -> Option<u16> {
        SchemeHostnamePort::from_str(&self.to)
            .ok()
            .map(|v| v.port())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_zone_config() {
        let yaml = r#"
name: my-zone
pods:
- name: client-agent
  expose-port: 3000,
  containers:
  - name: client-agent
    image: client-app
    ockam-ticket:
      attributes:
        - name: role
          value: client
      relay: true
    imagePullPolicy: Always
    args: ["client-agent", "${ENROLLMENT_TICKET}", "${ZONE_DOMAIN}-echo-agent"]
- name: echo-agent
  containers:
  - name: echo-agent
    image: echo-app
    ockam-ticket:
      attributes:
        - name: role
          value: echo
      relay: true
    imagePullPolicy: Always
    args: ["echo-agent", "${ENROLLMENT_TICKET}"]
  portal:
    attributes:
      - name: role
        value: echo
    tcp-outlets:
      - to: localhost:8080
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify strictly typed fields
        assert_eq!(config.name, "my-zone");
        assert_eq!(config.pods.len(), 2);
        assert_eq!(config.pods[0].name, "client-agent");
        assert_eq!(config.pods[0].containers[0].image, "client-app");

        // Check the portal field
        let portal = &config.pods[1].portals;
        assert_eq!(portal.inlets.len(), 0);
        assert_eq!(portal.outlets.len(), 1);
        let outlet = &portal.outlets[0];
        assert_eq!(outlet.to, "localhost:8080");
        assert_eq!(outlet.name, None);
        assert_eq!(outlet.to, "localhost:8080");
        assert_eq!(
            portal
                .other_fields
                .get("attributes")
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );

        // Check the deepest dynamic field
        let ticket = config.pods[0].containers[0]
            .other_fields
            .get("ockam-ticket")
            .unwrap();

        let attributes = ticket.get("attributes").unwrap().as_array().unwrap();
        assert_eq!(attributes.len(), 1);

        let attribute = &attributes[0];
        assert_eq!(attribute.get("name").unwrap().as_str().unwrap(), "role");
        assert_eq!(attribute.get("value").unwrap().as_str().unwrap(), "client");
        assert!(ticket.get("relay").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_get_local_images_names() {
        let zone_config = ZoneConfig {
            name: "xyz".to_string(),
            pods: vec![
                Pod {
                    name: "pod1".to_string(),
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: "local-image".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "local-image:tag".to_string(),
                            other_fields: HashMap::new(),
                        },
                    ],
                    portals: Default::default(),
                    other_fields: HashMap::new(),
                },
                Pod {
                    name: "pod2".to_string(),
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: "registry.com/remote-image".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "username/image:1.0".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
                            name: "efg".to_string(),
                            image: "another-local".to_string(),
                            other_fields: HashMap::new(),
                        },
                    ],
                    portals: Default::default(),
                    other_fields: HashMap::new(),
                },
                Pod {
                    name: "pod3".to_string(),
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: " trimmed-local ".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "".to_string(), // Empty image name
                            other_fields: HashMap::new(),
                        },
                    ],
                    portals: Default::default(),
                    other_fields: HashMap::new(),
                },
            ],
        };

        let local_images = zone_config.get_local_images_names();

        assert_eq!(local_images.len(), 4);
        assert!(local_images.contains(&"local-image".to_string()));
        assert!(local_images.contains(&"local-image:tag".to_string()));
        assert!(local_images.contains(&"another-local".to_string()));
        assert!(local_images.contains(&"trimmed-local".to_string()));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ZoneConfig {
            name: "valid-zone".to_string(),
            pods: vec![Pod {
                name: "pod1".to_string(),
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_long_zone_name() {
        let config = ZoneConfig {
            name: "this-zone-name-is-too-long".to_string(),
            pods: vec![Pod {
                name: "pod1".to_string(),
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Zone name exceeds 10 characters"));
    }

    #[test]
    fn test_validate_long_pod_name() {
        let config = ZoneConfig {
            name: "zone".to_string(),
            pods: vec![Pod {
                name: "pod-name-too-long".to_string(),
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Pod name 'pod-name-too-long' exceeds 10 characters"));
    }

    #[test]
    fn test_validate_long_container_name() {
        let config = ZoneConfig {
            name: "zone".to_string(),
            pods: vec![Pod {
                name: "pod1".to_string(),
                containers: vec![Container {
                    name: "container-name-is-too-long".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Container name 'container-name-is-too-long' exceeds 10 characters"));
    }

    #[test]
    fn test_validate_duplicate_names() {
        // Test duplicate pod names
        let config_duplicate_pods = ZoneConfig {
            name: "zone".to_string(),
            pods: vec![
                Pod {
                    name: "pod1".to_string(),
                    containers: vec![Container {
                        name: "container1".to_string(),
                        image: "image1".to_string(),
                        other_fields: HashMap::new(),
                    }],
                    portals: Default::default(),
                    other_fields: HashMap::new(),
                },
                Pod {
                    name: "pod1".to_string(), // Duplicate pod name
                    containers: vec![Container {
                        name: "container2".to_string(),
                        image: "image2".to_string(),
                        other_fields: HashMap::new(),
                    }],
                    portals: Default::default(),
                    other_fields: HashMap::new(),
                },
            ],
        };

        let result = config_duplicate_pods.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate pod name 'pod1' found"));

        // Test duplicate container names within a pod
        let config_duplicate_containers = ZoneConfig {
            name: "zone".to_string(),
            pods: vec![Pod {
                name: "pod1".to_string(),
                containers: vec![
                    Container {
                        name: "container1".to_string(),
                        image: "image1".to_string(),
                        other_fields: HashMap::new(),
                    },
                    Container {
                        name: "container1".to_string(), // Duplicate container name
                        image: "image2".to_string(),
                        other_fields: HashMap::new(),
                    },
                ],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        let result = config_duplicate_containers.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate container name 'container1' found in pod 'pod1'"));
    }

    #[test]
    fn test_validate_multiple_issues() {
        let config = ZoneConfig {
            name: "long-zone-name".to_string(),
            pods: vec![Pod {
                name: "long-pod-name".to_string(),
                containers: vec![Container {
                    name: "container-name-is-too-long".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Default::default(),
                other_fields: HashMap::new(),
            }],
        };

        let result = config.validate();
        assert!(result.is_err());
        // It should fail on the first validation (zone name)
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Zone name exceeds 10 characters"));
    }

    #[test]
    fn test_parse_yaml_with_portals() {
        let yaml = r#"
name: test-zone
pods:
- name: pod1
  containers:
  - name: app
    image: app-image
  portals:
    inlets:
    - name: web
      from: external:8080
    - from: external:9000
    outlets:
    - name: db
      to: postgres:5432
    - to: redis:6379
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify portals are correctly parsed
        let pod = &config.pods[0];
        assert_eq!(pod.portals.inlets.len(), 2);
        assert_eq!(pod.portals.outlets.len(), 2);

        // Verify inlet fields
        assert_eq!(pod.portals.inlets[0].name, Some("web".to_string()));
        assert_eq!(pod.portals.inlets[0].from, "external:8080");
        assert_eq!(pod.portals.inlets[1].name, None);
        assert_eq!(pod.portals.inlets[1].from, "external:9000");

        // Verify outlet fields
        assert_eq!(pod.portals.outlets[0].name, Some("db".to_string()));
        assert_eq!(pod.portals.outlets[0].to, "postgres:5432");
        assert_eq!(pod.portals.outlets[1].name, None);
        assert_eq!(pod.portals.outlets[1].to, "redis:6379");
    }

    #[test]
    fn test_parse_yaml_with_tcp_alias() {
        let yaml = r#"
name: test-zone
pods:
- name: pod1
  containers:
  - name: app
    image: app-image
  portals:
    tcp-inlets:
    - name: http
      from: external:80
    tcp-outlets:
    - name: api
      to: service:3000
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify alias works for tcp-inlets/outlets
        let pod = &config.pods[0];
        assert_eq!(pod.portals.inlets.len(), 1);
        assert_eq!(pod.portals.outlets.len(), 1);

        assert_eq!(pod.portals.inlets[0].name, Some("http".to_string()));
        assert_eq!(pod.portals.inlets[0].from, "external:80");

        assert_eq!(pod.portals.outlets[0].name, Some("api".to_string()));
        assert_eq!(pod.portals.outlets[0].to, "service:3000");
    }

    #[test]
    fn test_default_portals() {
        let yaml = r#"
name: test-zone
pods:
- name: pod1
  containers:
  - name: app
    image: app-image
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify default empty portals
        let pod = &config.pods[0];
        assert!(pod.portals.inlets.is_empty());
        assert!(pod.portals.outlets.is_empty());
    }

    #[test]
    fn test_other_fields_in_portals() {
        let yaml = r#"
name: test-zone
pods:
- name: pod1
  containers:
  - name: app
    image: app-image
  portals:
    inlets:
    - name: web
      from: external:8080
      protocol: http
      custom_field: value1
    outlets:
    - to: service:5000
      secure: true
      custom_field: value2
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify other_fields in inlets/outlets
        let pod = &config.pods[0];
        let inlet = &pod.portals.inlets[0];
        let outlet = &pod.portals.outlets[0];

        // Check custom fields are preserved in other_fields
        assert!(inlet.other_fields.contains_key("protocol"));
        assert_eq!(inlet.other_fields["protocol"], "http");
        assert!(inlet.other_fields.contains_key("custom_field"));
        assert_eq!(inlet.other_fields["custom_field"], "value1");

        assert!(outlet.other_fields.contains_key("secure"));
        assert_eq!(outlet.other_fields["secure"], true);
        assert!(outlet.other_fields.contains_key("custom_field"));
        assert_eq!(outlet.other_fields["custom_field"], "value2");
    }

    #[test]
    fn test_serialization_with_portals() {
        // Create a config with portals
        let mut config = ZoneConfig {
            name: "zone".to_string(),
            pods: vec![Pod {
                name: "pod1".to_string(),
                containers: vec![Container {
                    name: "app".to_string(),
                    image: "image1".to_string(),
                    other_fields: HashMap::new(),
                }],
                portals: Portals {
                    inlets: vec![
                        Inlet {
                            name: Some("api".to_string()),
                            from: "external:3000".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Inlet {
                            name: None,
                            from: "ext:8080".to_string(),
                            other_fields: HashMap::new(),
                        },
                    ],
                    outlets: vec![Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        other_fields: HashMap::new(),
                    }],
                    other_fields: Default::default(),
                },
                other_fields: HashMap::new(),
            }],
        };

        // Add some other_fields
        let mut custom_field = HashMap::new();
        custom_field.insert("protocol".to_string(), Value::String("tcp".to_string()));
        config.pods[0].portals.inlets[0].other_fields = custom_field;

        // Serialize and deserialize
        let serialized = serde_yaml::to_string(&config).unwrap();
        let deserialized: ZoneConfig = serde_yaml::from_str(&serialized).unwrap();

        // Verify everything matches
        assert_eq!(deserialized.pods[0].portals.inlets.len(), 2);
        assert_eq!(deserialized.pods[0].portals.outlets.len(), 1);

        let inlet1 = &deserialized.pods[0].portals.inlets[0];
        assert_eq!(inlet1.name, Some("api".to_string()));
        assert_eq!(inlet1.from, "external:3000");
        assert_eq!(inlet1.other_fields["protocol"], "tcp");

        let inlet2 = &deserialized.pods[0].portals.inlets[1];
        assert_eq!(inlet2.name, None);
        assert_eq!(inlet2.from, "ext:8080");

        let outlet = &deserialized.pods[0].portals.outlets[0];
        assert_eq!(outlet.name, Some("db".to_string()));
        assert_eq!(outlet.to, "postgres:5432");
    }

    #[test]
    fn test_get_main_pod() {
        // Test case: only one pod exists
        let yaml_single_pod = r#"
        name: test-zone
        pods:
        - name: single-pod
          containers:
          - name: app
            image: app-image
        "#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml_single_pod).unwrap();
        let main_pod = config.get_main_pod().unwrap();
        assert_eq!(main_pod.name, "single-pod");

        // Test case: multiple pods, one named "main"
        let yaml_with_main = r#"
        name: test-zone
        pods:
        - name: pod1
          containers:
          - name: app1
            image: app1-image
        - name: main
          containers:
          - name: app2
            image: app2-image
        - name: pod3
          containers:
          - name: app3
            image: app3-image
        "#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml_with_main).unwrap();
        let main_pod = config.get_main_pod().unwrap();
        assert_eq!(main_pod.name, "main");

        // Test case: multiple pods, none named "main"
        let yaml_without_main = r#"
        name: test-zone
        pods:
        - name: pod1
          containers:
          - name: app1
            image: app1-image
        - name: pod2
          containers:
          - name: app2
            image: app2-image
        "#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml_without_main).unwrap();
        let err = config.get_main_pod().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Multiple pods defined, but none is named 'main'"
        );

        // Test case: no pods
        let yaml_no_pods = r#"
        name: test-zone
        pods: []
        "#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml_no_pods).unwrap();
        let err = config.get_main_pod().unwrap_err();
        assert_eq!(err.to_string(), "No pods defined in zone configuration");
    }

    #[test]
    fn test_pod_get_outlets_with_repl() {
        // Setup pod with multiple outlets including a "repl" outlet
        let pod = Pod {
            name: "test-pod".to_string(),
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                other_fields: HashMap::new(),
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        other_fields: HashMap::new(),
                    },
                    Outlet {
                        name: Some("repl".to_string()),
                        to: "console:1234".to_string(),
                        other_fields: HashMap::new(),
                    },
                    Outlet {
                        name: Some("cache".to_string()),
                        to: "redis:6379".to_string(),
                        other_fields: HashMap::new(),
                    },
                ],
                other_fields: HashMap::new(),
            },
            other_fields: HashMap::new(),
        };

        let outlets = pod.get_outlets();

        // The "repl" outlet should be separated
        assert!(outlets.repl.is_some());
        let repl = outlets.repl.unwrap();
        assert_eq!(repl.name, Some("repl".to_string()));
        assert_eq!(repl.to, "console:1234");

        // The rest vector should have the other two outlets
        assert_eq!(outlets.rest.len(), 2);
        assert!(outlets
            .rest
            .iter()
            .any(|o| o.name == Some("db".to_string())));
        assert!(outlets
            .rest
            .iter()
            .any(|o| o.name == Some("cache".to_string())));
    }

    #[test]
    fn test_pod_get_outlets_multiple_yaml() {
        let config = r"
name: example05
pods:
  - name: main-pod
    containers:
      - name: main
        image: main
        args: [localhost:9000, localhost:9001]
    portals:
      outlets:
        - name: repl
          to: localhost:9000
        - name: http
          to: localhost:9001
          ";

        let parsed = serde_yaml::from_str::<ZoneConfig>(config).unwrap();
        let pod = &parsed.pods[0];
        let outlets = pod.get_outlets();
        assert_eq!(
            outlets.repl.as_ref().unwrap().name,
            Some("repl".to_string())
        );
        assert_eq!(outlets.repl.as_ref().unwrap().to, "localhost:9000");
        assert_eq!(outlets.rest.len(), 1);
        assert_eq!(outlets.rest[0].name, Some("http".to_string()));
        assert_eq!(outlets.rest[0].to, "localhost:9001");
    }

    #[test]
    fn test_pod_get_outlets_single_outlet() {
        // Setup pod with a single outlet (not named "repl")
        let pod = Pod {
            name: "test-pod".to_string(),
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                other_fields: HashMap::new(),
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![Outlet {
                    name: Some("single".to_string()),
                    to: "service:8080".to_string(),
                    other_fields: HashMap::new(),
                }],
                other_fields: HashMap::new(),
            },
            other_fields: HashMap::new(),
        };

        let outlets = pod.get_outlets();

        // When there's only one outlet, it should be used as the repl
        assert!(outlets.repl.is_some());
        let repl = outlets.repl.unwrap();
        assert_eq!(repl.name, Some("single".to_string()));
        assert_eq!(repl.to, "service:8080");

        // The rest vector should be empty
        assert_eq!(outlets.rest.len(), 0);
    }

    #[test]
    fn test_pod_get_outlets_unnamed_outlets() {
        // Setup pod with unnamed outlets
        let pod = Pod {
            name: "test-pod".to_string(),
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                other_fields: HashMap::new(),
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: None,
                        to: "service1:8080".to_string(),
                        other_fields: HashMap::new(),
                    },
                    Outlet {
                        name: None,
                        to: "service2:9090".to_string(),
                        other_fields: HashMap::new(),
                    },
                ],
                other_fields: HashMap::new(),
            },
            other_fields: HashMap::new(),
        };

        let outlets = pod.get_outlets();

        // No "repl" outlet exists
        assert!(outlets.repl.is_none());

        // The rest vector should have the other outlet
        assert_eq!(outlets.rest.len(), 2);
        assert_eq!(outlets.rest[0].to, "service1:8080");
        assert_eq!(outlets.rest[1].to, "service2:9090");
    }

    #[test]
    fn test_pod_get_outlets_no_outlets() {
        // Setup pod with no outlets
        let pod = Pod {
            name: "test-pod".to_string(),
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                other_fields: HashMap::new(),
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![],
                other_fields: HashMap::new(),
            },
            other_fields: HashMap::new(),
        };

        let outlets = pod.get_outlets();

        // No outlets means no repl
        assert!(outlets.repl.is_none());

        // The rest vector should be empty
        assert_eq!(outlets.rest.len(), 0);
    }

    #[test]
    fn test_pod_get_outlets_multiple_no_repl() {
        // Setup pod with multiple outlets but no "repl"
        let pod = Pod {
            name: "test-pod".to_string(),
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                other_fields: HashMap::new(),
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        other_fields: HashMap::new(),
                    },
                    Outlet {
                        name: Some("cache".to_string()),
                        to: "redis:6379".to_string(),
                        other_fields: HashMap::new(),
                    },
                ],
                other_fields: HashMap::new(),
            },
            other_fields: HashMap::new(),
        };

        let outlets = pod.get_outlets();

        // No "repl" outlet exists, and since there are multiple outlets,
        // none gets chosen for repl (first one doesn't become repl)
        assert!(outlets.repl.is_none());

        // The rest vector should have both outlets
        assert_eq!(outlets.rest.len(), 2);
        assert!(outlets
            .rest
            .iter()
            .any(|o| o.name == Some("db".to_string())));
        assert!(outlets
            .rest
            .iter()
            .any(|o| o.name == Some("cache".to_string())));
    }
}
