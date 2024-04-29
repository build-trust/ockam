use ockam_multiaddr::MultiAddr;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum Throughput {
    #[default]
    Unlimited,
    Bytes(u32),
}

impl<'de> Deserialize<'de> for Throughput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?
            .to_lowercase()
            .replace(['_', ' '], "");
        if s == "unlimited" {
            Ok(Throughput::Unlimited)
        } else {
            let number: u32 = s
                .trim_end_matches(|c: char| !c.is_numeric())
                .parse()
                .map_err(serde::de::Error::custom)?;
            let unit = s.trim_start_matches(|c: char| c.is_numeric());
            Ok(Throughput::Bytes(match unit {
                "gbits" => number * (1_000_000_000) / 8,
                "mbits" => number * (1_000_000) / 8,
                "kbits" => number * 1_000 / 8,
                "bits" => number / 8,
                "" => number / 8,
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "Invalid throughput unit: {unit}"
                    )));
                }
            }))
        }
    }
}

fn default_project() -> String {
    "/project/default".to_string()
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    peak_portals: usize,
    #[serde(default)]
    peak_relays: usize,
    #[serde(default)]
    ramp_up: usize,
    #[serde(default)]
    throughput: Throughput,
    #[serde(default = "default_project")]
    project: String,
}

impl Config {
    pub fn sample_configs() -> String {
        r#"
peak_portals = 1_000
peak_relays = 100
ramp_up = 60
throughput = "unlimited"
project = "/project/default"

peak_portals = 1_000
peak_relays = 100
ramp_up = 120
throughput = "10 mbits"
project = "/project/default"
"#
        .to_string()
    }

    pub fn parse(path: &std::path::Path) -> Result<Config, toml::de::Error> {
        let toml = std::fs::read_to_string(path).expect("Failed to read configuration file");
        toml::from_str(&toml)
    }

    /// Returns the current progress of the ramp up in the range [0, 1]
    fn progress(&self, elapsed_seconds: f32) -> f32 {
        (elapsed_seconds / self.ramp_up as f32).max(0.0).min(1.0)
    }

    pub fn calculate_relays(&self, elapsed_seconds: f32) -> usize {
        ((self.peak_relays as f32 * self.progress(elapsed_seconds)).ceil() as usize).max(1)
    }

    pub fn calculate_portals(&self, elapsed_seconds: f32) -> usize {
        (self.peak_portals as f32 * self.progress(elapsed_seconds)).ceil() as usize
    }

    pub fn project_addr(&self) -> MultiAddr {
        self.project.parse().expect("Invalid project address")
    }

    pub fn throughput(&self) -> Throughput {
        self.throughput.clone()
    }
}

#[test]
pub fn sample_config() -> Result<(), toml::de::Error> {
    let toml = r#"
peak_portals = 100
peak_relays = 50
ramp_up = 60
throughput = "unlimited"
project = "/project/default"
"#;

    let config: Config = toml::from_str(toml)?;

    assert_eq!(config.peak_portals, 100);
    assert_eq!(config.peak_relays, 50);
    assert_eq!(config.ramp_up, 60);
    assert_eq!(config.throughput, Throughput::Unlimited);
    assert_eq!(config.project, "/project/default");
    assert_eq!(config.calculate_relays(0.0), 1);
    assert_eq!(config.calculate_portals(0.0), 0);
    assert_eq!(config.calculate_relays(30.0), 25);
    assert_eq!(config.calculate_portals(30.0), 50);
    assert_eq!(config.calculate_relays(60.0), 50);
    assert_eq!(config.calculate_portals(60.0), 100);
    assert_eq!(config.calculate_relays(1000.0), 50);
    assert_eq!(config.calculate_portals(1000.0), 100);

    let toml = r#"
peak_portals = 20_000
peak_relays = 1_000
ramp_up = 120
throughput = "10 mbits"
project = "/project/default"
"#;

    let config: Config = toml::from_str(toml)?;

    assert_eq!(config.ramp_up, 120);
    assert_eq!(config.throughput, Throughput::Bytes(1250000));

    Ok(())
}
