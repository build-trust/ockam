use std::fmt::Formatter;
use std::io::Write;
use std::{fmt::Display, fs::File, io::Read, path::Path, str::FromStr};

use anyhow::anyhow;

use crate::config::build_config_path;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodeConfigVersion {
    V0,
    V1,
}

#[allow(unused)]
impl NodeConfigVersion {
    const FILE_NAME: &'static str = "version";

    fn latest() -> Self {
        Self::V1
    }

    fn load(config_dir: &Path) -> anyhow::Result<Self> {
        let version_path = config_dir.join(Self::FILE_NAME);
        let version = if version_path.exists() {
            let mut version_file = File::open(version_path)?;
            let mut version = String::new();
            version_file.read_to_string(&mut version)?;
            NodeConfigVersion::from_str(&version)?
        } else {
            Self::V0
        };
        debug!(%version, "Loaded config");
        version.upgrade(config_dir)
    }

    fn upgrade(&self, config_dir: &Path) -> anyhow::Result<Self> {
        let from = self;
        let mut final_version = from.clone();

        // Iter through all the versions between `from` and `to`
        let f = from.to_string().parse::<u8>()?;
        let mut t = f + 1;
        while let Ok(ref to) = Self::from_str(&t.to_string()) {
            debug!(%from, %to, "Upgrading config");
            final_version = to.clone();
            #[allow(clippy::single_match)]
            match (from, to) {
                (Self::V0, Self::V1) => {
                    if let (Some(old_config_name), Some(new_config_name)) =
                        (from.state_config_name(), to.state_config_name())
                    {
                        let old_config_path = build_config_path(config_dir, old_config_name);
                        // If old config path exists, copy to new config path and keep the old one
                        if old_config_path.exists() {
                            let new_config_path = build_config_path(config_dir, new_config_name);
                            std::fs::copy(old_config_path, new_config_path)?;
                        }
                        // Create the version file if doesn't exists
                        Self::set_version(config_dir, to)?;
                    }
                }
                _ => {}
            }
            t += 1;
        }
        Ok(final_version)
    }

    fn dirs(&self) -> &'static [&'static str] {
        match self {
            Self::V0 => &["config"],
            Self::V1 => &["state", "commands"],
        }
    }

    fn state_config_name(&self) -> Option<&'static str> {
        match self {
            Self::V0 => Some("config"),
            Self::V1 => Some("state"),
        }
    }

    fn commands_config_name(&self) -> Option<&'static str> {
        match self {
            Self::V0 => None,
            Self::V1 => Some("commands"),
        }
    }

    fn set_version(config_dir: &Path, version: &NodeConfigVersion) -> anyhow::Result<()> {
        let version_path = config_dir.join(Self::FILE_NAME);
        let mut version_file = File::create(version_path)?;
        version_file.write_all(version.to_string().as_bytes())?;
        Ok(())
    }
}

impl Display for NodeConfigVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NodeConfigVersion::V0 => "0",
            NodeConfigVersion::V1 => "1",
        })
    }
}

impl FromStr for NodeConfigVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::V0),
            "1" => Ok(Self::V1),
            _ => Err(anyhow!("Unknown version: {}", s)),
        }
    }
}
