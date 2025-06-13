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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pod {
    pub name: String,
    #[serde(default)]
    pub public: bool,
    pub containers: Vec<Container>,
    #[serde(default, alias = "portal")]
    pub portals: Portals,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Portals {
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        default,
        alias = "inlet",
        alias = "tcp-inlets",
        alias = "tcp-inlet"
    )]
    pub inlets: Vec<Inlet>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        default,
        alias = "outlet",
        alias = "tcp-outlets",
        alias = "tcp-outlets"
    )]
    pub outlets: Vec<Outlet>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Inlet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub from: String,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Outlet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_name: Option<String>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Container {
    pub name: String,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Env>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Env {
    ListOfMaps(Vec<HashMap<String, Value>>),
    MapOfKeyValues(HashMap<String, String>),
}

impl ZoneConfig {
    const MAIN_POD_NAME: &'static str = "main-pod";

    pub fn from_contents(content: &str) -> Result<Self, miette::Error> {
        let mut _self = if content.starts_with("{") {
            serde_json::from_str::<Self>(content)
                .map_err(|e| miette::miette!(format!("Failed to parse JSON zone config: {}", e)))
        } else {
            serde_yaml::from_str::<Self>(content)
                .map_err(|e| miette::miette!(format!("Failed to parse YAML zone config: {}", e)))
        }?;
        _self.validate()?;
        _self.fill_in_defaults()?;
        _self.transform_env_vars()?;
        Ok(_self)
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, miette::Error> {
        let content = std::fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err(format!(
                "Failed to read zone config file at {}",
                path.as_ref().display()
            ))?;
        Self::from_contents(&content)
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

    fn fill_in_defaults(&mut self) -> Result<(), miette::Error> {
        let mut http = None;
        let mut logs = None;
        let main_pod = self.get_main_pod()?.clone();
        for outlet in &main_pod.portals.outlets {
            match outlet.name.as_deref() {
                Some("http") => http = Some(outlet.clone()),
                Some("logs") => logs = Some(outlet.clone()),
                _ => {}
            }
        }
        if http.is_none() {
            self.pods
                .iter_mut()
                .filter(|pod| pod.name == main_pod.name)
                .for_each(|pod| {
                    pod.portals.outlets.push(Outlet {
                        name: Some("http".to_string()),
                        to: "localhost:8000".to_string(),
                        ..Default::default()
                    })
                });
        }
        if logs.is_none() {
            self.pods
                .iter_mut()
                .filter(|pod| pod.name == main_pod.name)
                .for_each(|pod| {
                    pod.portals.outlets.push(Outlet {
                        name: Some("logs".to_string()),
                        to: "localhost:3000".to_string(),
                        pod_name: Some("logs-pod".to_string()),
                        ..Default::default()
                    })
                });
        }
        Ok(())
    }

    fn transform_env_vars(&mut self) -> Result<(), miette::Error> {
        for pod in &mut self.pods {
            for container in &mut pod.containers {
                if let Some(env) = &mut container.env {
                    match env {
                        Env::ListOfMaps(list) => {
                            let mut maps = vec![];
                            for map in list.drain(..) {
                                // If the hashmap has a single item and value is a string,
                                // process it as a key-value env var
                                if map.len() == 1 {
                                    if let Some((key, Value::String(value_str))) = map.iter().next()
                                    {
                                        let transformed_item =
                                            Self::create_env_item(key, value_str);
                                        maps.push(transformed_item);
                                    }
                                }
                                // Otherwise, leave as is
                                else {
                                    maps.push(map);
                                }
                            }
                            *env = Env::ListOfMaps(maps);
                        }
                        Env::MapOfKeyValues(map) => {
                            let mut transformed_list = Vec::new();
                            for (key, value) in map.iter() {
                                transformed_list.push(Self::create_env_item(key, value));
                            }
                            *env = Env::ListOfMaps(transformed_list);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn create_env_item(key: &str, value: &str) -> HashMap<String, Value> {
        let mut env_item = HashMap::new();
        if let Some(secret_key) = Self::extract_secret_key(&Value::String(value.to_string())) {
            env_item.insert("name".to_string(), Value::String(key.to_string()));
            env_item.insert(
                "valueFrom".to_string(),
                serde_json::json!({
                    "secretKeyRef": {
                        "name": "secret",
                        "key": secret_key
                    }
                }),
            );
        } else {
            env_item.insert(key.to_string(), Value::String(value.to_string()));
        }
        env_item
    }

    fn extract_secret_key(value: &Value) -> Option<String> {
        if let Value::String(s) = value {
            if let Some(stripped) = s.strip_prefix("secrets.") {
                return Some(stripped.to_string());
            }
        }
        None
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

    /// Returns the name of the main pod, which is either:
    /// - The only pod if there's just one pod
    /// - The main pod if it exists
    /// - Error otherwise
    pub fn get_main_pod(&self) -> miette::Result<&Pod> {
        if self.pods.is_empty() {
            return Err(miette::miette!("No pods defined in zone configuration"));
        }
        if self.pods.len() == 1 {
            // If there's only one pod, return it
            Ok(&self.pods[0])
        } else {
            // Try to find the main pod
            self.pods
                .iter()
                .find(|pod| pod.name == Self::MAIN_POD_NAME)
                .ok_or_else(|| {
                    miette::miette!(format!(
                        "Multiple pods defined, but none is named '{}'",
                        Self::MAIN_POD_NAME
                    ))
                })
        }
    }

    pub fn get_http_url(&self, cluster_name: &str) -> Option<String> {
        if self.get_main_pod().ok()?.public {
            Some(format!(
                "https://{}-{}.ai.ockam.network",
                cluster_name, self.name
            ))
        } else {
            None
        }
    }
}

impl Pod {
    pub fn get_outlets(&self) -> PodOutlets {
        let mut repl = None;
        let mut http = Outlet::default();
        let mut logs = Outlet::default();
        let mut rest = Vec::new();

        for outlet in &self.portals.outlets {
            match outlet.name.as_deref() {
                Some("repl") => repl = Some(outlet.clone()),
                Some("http") => http = outlet.clone(),
                Some("logs") => logs = outlet.clone(),
                _ => rest.push(outlet.clone()),
            }
        }

        // If there is only one unnamed outlet, return it as the repl
        if repl.is_none() && rest.len() == 1 && rest[0].name.is_none() {
            let outlet = rest.remove(0);
            repl = Some(outlet);
        }

        PodOutlets {
            repl,
            http,
            logs,
            rest,
        }
    }
}

pub struct PodOutlets {
    pub repl: Option<Outlet>,
    pub http: Outlet,
    pub logs: Outlet,
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
- name: main-pod
  expose-port: 3000,
  containers:
  - name: client
    image: client-app
    ockam-ticket:
      attributes:
        - name: role
          value: client
      relay: true
    imagePullPolicy: Always
    args: ["client-agent", "${ENROLLMENT_TICKET}", "${ZONE_DOMAIN}-echo-agent"]
- name: echo
  containers:
  - name: echo
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

        let config = ZoneConfig::from_contents(yaml).unwrap();

        // Verify strictly typed fields
        assert_eq!(config.name, "my-zone");
        assert_eq!(config.pods.len(), 2);
        assert_eq!(config.pods[0].name, "main-pod");
        assert_eq!(config.pods[0].containers[0].image, "client-app");
        assert_eq!(config.pods[0].portals.outlets.len(), 2);

        // Check the portal field in the echo pod (not main)
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
                    public: false,
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: "local-image".to_string(),
                            ..Default::default()
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "local-image:tag".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                Pod {
                    name: "pod2".to_string(),
                    public: false,
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: "registry.com/remote-image".to_string(),
                            ..Default::default()
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "username/image:1.0".to_string(),
                            ..Default::default()
                        },
                        Container {
                            name: "efg".to_string(),
                            image: "another-local".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                Pod {
                    name: "pod3".to_string(),
                    public: false,
                    containers: vec![
                        Container {
                            name: "abc".to_string(),
                            image: " trimmed-local ".to_string(),
                            ..Default::default()
                        },
                        Container {
                            name: "cde".to_string(),
                            image: "".to_string(), // Empty image name
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
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
                public: false,
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
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
                public: false,
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
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
                public: false,
                containers: vec![Container {
                    name: "abc".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
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
                public: false,
                containers: vec![Container {
                    name: "container-name-is-too-long".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
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
                    public: false,
                    containers: vec![Container {
                        name: "container1".to_string(),
                        image: "image1".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                Pod {
                    name: "pod1".to_string(), // Duplicate pod name
                    public: false,
                    containers: vec![Container {
                        name: "container2".to_string(),
                        image: "image2".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
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
                public: false,
                containers: vec![
                    Container {
                        name: "container1".to_string(),
                        image: "image1".to_string(),
                        ..Default::default()
                    },
                    Container {
                        name: "container1".to_string(), // Duplicate container name
                        image: "image2".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
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
                public: false,
                containers: vec![Container {
                    name: "container-name-is-too-long".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
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

        let config = ZoneConfig::from_contents(yaml).unwrap();

        // Verify portals are correctly parsed
        let pod = &config.pods[0];
        assert_eq!(pod.portals.inlets.len(), 2);
        assert_eq!(pod.portals.outlets.len(), 4);

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

        let config = ZoneConfig::from_contents(yaml).unwrap();

        // Verify alias works for tcp-inlets/outlets
        let pod = &config.pods[0];
        assert_eq!(pod.portals.inlets.len(), 1);
        assert_eq!(pod.portals.outlets.len(), 3);

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

        let config = ZoneConfig::from_contents(yaml).unwrap();

        // Verify default portals
        let pod = &config.pods[0];
        assert!(pod.portals.inlets.is_empty());
        assert_eq!(pod.portals.outlets.len(), 2);
        assert_eq!(pod.portals.outlets[0].name, Some("http".to_string()));
        assert_eq!(pod.portals.outlets[1].name, Some("logs".to_string()));
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

        let config = ZoneConfig::from_contents(yaml).unwrap();

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
                public: false,
                containers: vec![Container {
                    name: "app".to_string(),
                    image: "image1".to_string(),
                    ..Default::default()
                }],
                portals: Portals {
                    inlets: vec![
                        Inlet {
                            name: Some("api".to_string()),
                            from: "external:3000".to_string(),
                            ..Default::default()
                        },
                        Inlet {
                            name: None,
                            from: "ext:8080".to_string(),
                            ..Default::default()
                        },
                    ],
                    outlets: vec![Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        ..Default::default()
                    }],
                    other_fields: Default::default(),
                },
                ..Default::default()
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
        // only one pod exists
        let yaml_single_pod = r#"
        name: test-zone
        pods:
        - name: single-pod
          containers:
          - name: app
            image: app-image
        "#;

        let config = ZoneConfig::from_contents(yaml_single_pod).unwrap();
        let main_pod = config.get_main_pod().unwrap();
        assert_eq!(main_pod.name, "single-pod");

        // multiple pods, one named "main-pod"
        let yaml_with_main = r#"
        name: test-zone
        pods:
        - name: pod1
          containers:
          - name: app1
            image: app1-image
        - name: main-pod
          containers:
          - name: app2
            image: app2-image
        - name: pod3
          containers:
          - name: app3
            image: app3-image
        "#;

        let config = ZoneConfig::from_contents(yaml_with_main).unwrap();
        let main_pod = config.get_main_pod().unwrap();
        assert_eq!(main_pod.name, ZoneConfig::MAIN_POD_NAME);

        // multiple pods, none named "main-pod"
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

        let config = ZoneConfig::from_contents(yaml_without_main);
        assert!(config.is_err());

        // no pods
        let yaml_no_pods = r#"
        name: test-zone
        pods: []
        "#;

        let config = ZoneConfig::from_contents(yaml_no_pods);
        assert!(config.is_err());
    }

    #[test]
    fn test_pod_get_outlets_with_repl() {
        // Setup pod with multiple outlets including a "repl" outlet
        let pod = Pod {
            name: "test-pod".to_string(),
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        ..Default::default()
                    },
                    Outlet {
                        name: Some("repl".to_string()),
                        to: "console:1234".to_string(),
                        ..Default::default()
                    },
                    Outlet {
                        name: Some("cache".to_string()),
                        to: "redis:6379".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
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
        - name: custom
          to: localhost:9001
          ";

        let parsed = ZoneConfig::from_contents(config).unwrap();
        let pod = &parsed.pods[0];
        let outlets = pod.get_outlets();
        assert_eq!(
            outlets.repl.as_ref().unwrap().name,
            Some("repl".to_string())
        );
        assert_eq!(outlets.repl.as_ref().unwrap().to, "localhost:9000");
        assert_eq!(outlets.rest.len(), 1);
        assert_eq!(outlets.rest[0].name, Some("custom".to_string()));
        assert_eq!(outlets.rest[0].to, "localhost:9001");
    }

    #[test]
    fn test_pod_get_outlets_single_outlet_named() {
        // Setup pod with a single named outlet (not named "repl")
        let pod = Pod {
            name: "test-pod".to_string(),
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![Outlet {
                    name: Some("single".to_string()),
                    to: "service:8080".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        let outlets = pod.get_outlets();

        // When there's only one named outlet, it should not be used as the repl
        assert!(outlets.repl.is_none());

        // The rest vector should be empty
        assert_eq!(outlets.rest.len(), 1);
        assert_eq!(outlets.rest.len(), 1);
        assert_eq!(outlets.rest[0].name, Some("single".to_string()));
        assert_eq!(outlets.rest[0].to, "service:8080");
    }

    #[test]
    fn test_pod_get_outlets_single_outlet_unnamed() {
        // Setup pod with a single unnamed outlet (not named "repl")
        let pod = Pod {
            name: "test-pod".to_string(),
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![Outlet {
                    name: None,
                    to: "service:8080".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };

        let outlets = pod.get_outlets();

        // When there's only one outlet, it should be used as the repl
        assert!(outlets.repl.is_some());
        let repl = outlets.repl.unwrap();
        assert_eq!(repl.name, None);
        assert_eq!(repl.to, "service:8080");

        // The rest vector should be empty
        assert_eq!(outlets.rest.len(), 0);
    }

    #[test]
    fn test_pod_get_outlets_unnamed_outlets() {
        // Setup pod with unnamed outlets
        let pod = Pod {
            name: "test-pod".to_string(),
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        to: "service1:8080".to_string(),
                        ..Default::default()
                    },
                    Outlet {
                        to: "service2:9090".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
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
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![],
                ..Default::default()
            },
            ..Default::default()
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
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: Some("db".to_string()),
                        to: "postgres:5432".to_string(),
                        ..Default::default()
                    },
                    Outlet {
                        name: Some("cache".to_string()),
                        to: "redis:6379".to_string(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
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

    #[test]
    fn test_pod_get_outlets_with_explicit_http_and_logs() {
        // Setup pod with http and logs outlets explicitly defined
        let pod = Pod {
            name: "test-pod".to_string(),
            public: false,
            containers: vec![Container {
                name: "app".to_string(),
                image: "app-image".to_string(),
                ..Default::default()
            }],
            portals: Portals {
                inlets: vec![],
                outlets: vec![
                    Outlet {
                        name: Some("http".to_string()),
                        to: "custom-host:8080".to_string(),
                        ..Default::default()
                    },
                    Outlet {
                        name: Some("logs".to_string()),
                        to: "logger:1234".to_string(),
                        pod_name: Some("logging-service".to_string()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let outlets = pod.get_outlets();

        // Verify the http outlet was correctly assigned
        assert_eq!(outlets.http.name, Some("http".to_string()));
        assert_eq!(outlets.http.to, "custom-host:8080");
        assert!(outlets.http.pod_name.is_none());

        // Verify the logs outlet was correctly assigned
        assert_eq!(outlets.logs.name, Some("logs".to_string()));
        assert_eq!(outlets.logs.to, "logger:1234");
        assert_eq!(outlets.logs.pod_name, Some("logging-service".to_string()));

        // Verify rest contains no items
        assert_eq!(outlets.rest.len(), 0);
    }

    #[test]
    fn test_parse_env_raw_format() {
        let yaml = r#"
        name: example007
        pods:
          - name: main-pod
            public: true
            containers:
              - name: main
                image: main
              - name: mcp
                image: ghcr.io/build-trust/mcp-proxy
                env:
                  - name: BRAVE_API_KEY
                    valueFrom:
                      secretKeyRef:
                        name: brave
                        key: key
                args:
                  ["--sse-port", "8001", "--pass-environment", "--", "npx", "-y", "@modelcontextprotocol/server-brave-search"]
        "#;

        let config = ZoneConfig::from_contents(yaml).unwrap();

        let mcp_container = config.pods[0]
            .containers
            .iter()
            .find(|c| c.name == "mcp")
            .unwrap();

        if let Some(Env::ListOfMaps(env_values)) = &mcp_container.env {
            assert_eq!(env_values.len(), 1);

            let env_item = &env_values[0];
            assert!(env_item.contains_key("name"));
            assert_eq!(
                env_item.get("name").unwrap().as_str().unwrap(),
                "BRAVE_API_KEY"
            );

            assert!(env_item.contains_key("valueFrom"));
            let value_from = env_item.get("valueFrom").unwrap().as_object().unwrap();
            let secret_key_ref = value_from.get("secretKeyRef").unwrap().as_object().unwrap();
            assert_eq!(
                secret_key_ref.get("name").unwrap().as_str().unwrap(),
                "brave"
            );
            assert_eq!(secret_key_ref.get("key").unwrap().as_str().unwrap(), "key");
        } else {
            panic!("Expected env to be parsed as Env::RawList");
        }
    }

    mod env_var_transformation {
        use super::*;

        #[test]
        fn test_transform_env_vars_from_map() {
            let input_yaml = r#"
            name: test-zone
            pods:
            - name: main-pod
              containers:
              - name: app
                image: app-image
                env:
                  API_KEY: "secrets.MY_API_KEY"
                  REGULAR_VAR: "regular-value"
            "#;

            let config = ZoneConfig::from_contents(input_yaml).unwrap();

            let container = &config.pods[0].containers[0];
            if let Some(Env::ListOfMaps(raw_env)) = &container.env {
                assert_eq!(raw_env.len(), 2);

                // Check API_KEY transformation to a secret reference
                let api_key = raw_env
                    .iter()
                    .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("API_KEY"))
                    .expect("API_KEY not found");

                let value_from = api_key
                    .get("valueFrom")
                    .expect("Missing valueFrom")
                    .as_object()
                    .unwrap();
                let secret_key_ref = value_from
                    .get("secretKeyRef")
                    .expect("Missing secretKeyRef")
                    .as_object()
                    .unwrap();

                assert_eq!(
                    secret_key_ref.get("name").unwrap().as_str().unwrap(),
                    "secret"
                );
                assert_eq!(
                    secret_key_ref.get("key").unwrap().as_str().unwrap(),
                    "MY_API_KEY"
                );

                // Check REGULAR_VAR is unchanged
                let regular_var = raw_env
                    .iter()
                    .find(|v| v.get("REGULAR_VAR").is_some())
                    .expect("REGULAR_VAR not found");

                assert_eq!(
                    regular_var.get("REGULAR_VAR").unwrap().as_str().unwrap(),
                    "regular-value"
                );
            } else {
                panic!("Expected Env::RawList variant after transformation");
            }
        }

        #[test]
        fn test_transform_env_vars_from_list_of_keyvalue() {
            let input_yaml = r#"
            name: test-zone
            pods:
            - name: main-pod
              containers:
              - name: app
                image: app-image
                env:
                - API_KEY: "secrets.MY_API_KEY"
                - REGULAR_VAR: "regular-value"
            "#;

            let config = ZoneConfig::from_contents(input_yaml).unwrap();
            let _config_as_yaml = serde_yaml::to_string(&config).unwrap();

            let container = &config.pods[0].containers[0];
            if let Some(Env::ListOfMaps(raw_env)) = &container.env {
                assert_eq!(raw_env.len(), 2);

                // Check API_KEY transformation to a secret reference
                let api_key = raw_env
                    .iter()
                    .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("API_KEY"))
                    .expect("API_KEY not found");

                let value_from = api_key
                    .get("valueFrom")
                    .expect("Missing valueFrom")
                    .as_object()
                    .unwrap();
                let secret_key_ref = value_from
                    .get("secretKeyRef")
                    .expect("Missing secretKeyRef")
                    .as_object()
                    .unwrap();

                assert_eq!(
                    secret_key_ref.get("name").unwrap().as_str().unwrap(),
                    "secret"
                );
                assert_eq!(
                    secret_key_ref.get("key").unwrap().as_str().unwrap(),
                    "MY_API_KEY"
                );

                // Check REGULAR_VAR is unchanged
                let regular_var = raw_env
                    .iter()
                    .find(|v| v.get("REGULAR_VAR").is_some())
                    .expect("REGULAR_VAR not found");

                assert_eq!(
                    regular_var.get("REGULAR_VAR").unwrap().as_str().unwrap(),
                    "regular-value"
                );
            } else {
                panic!("Expected Env::RawList variant after transformation");
            }
        }

        #[test]
        fn test_transform_env_vars_with_empty_env_field() {
            let yaml = r#"
            name: test-zone
            pods:
            - name: main-pod
              containers:
              - name: app
                image: app-image
                # No env specified
            "#;

            let config = ZoneConfig::from_contents(yaml).unwrap();
            let container = &config.pods[0].containers[0];
            assert!(container.env.is_none());
        }
    }
}
