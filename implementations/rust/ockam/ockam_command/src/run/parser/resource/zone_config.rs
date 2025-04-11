use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfig {
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
    pub image: String,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

impl ZoneConfig {
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, miette::Error> {
        let content = std::fs::read_to_string(path).into_diagnostic()?;
        if content.starts_with("{") {
            serde_json::from_str::<Self>(&content)
                .map_err(|e| miette::miette!(format!("Failed to parse JSON zone config: {}", e)))
        } else {
            serde_yaml::from_str::<Self>(&content)
                .map_err(|e| miette::miette!(format!("Failed to parse YAML zone config: {}", e)))
        }
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
      - attributes:
        - name: role
          value: echo
"#;

        let config = serde_yaml::from_str::<ZoneConfig>(yaml).unwrap();

        // Verify strictly typed fields
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
            pods: vec![
                Pod {
                    name: "pod1".to_string(),
                    containers: vec![
                        Container {
                            image: "local-image".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
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
                            image: "registry.com/remote-image".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
                            image: "username/image:1.0".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
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
                            image: " trimmed-local ".to_string(),
                            other_fields: HashMap::new(),
                        },
                        Container {
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
}
