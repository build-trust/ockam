use miette::{IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    expose-port: 3000,
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
}
