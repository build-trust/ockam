use crate::util::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct GetCommand {
    /// Name of the configuration value
    pub value: Option<String>,
}

impl GetCommand {
    pub fn run(cfg: &mut OckamConfig, command: GetCommand) {
        let msg = match command.value.as_deref() {
            Some("api-node") => cfg.api_node.clone(),
            // FIXME: needs to take an additional parameter
            // Some("log-path") => cfg.log_path.to_str().unwrap().to_owned(),
            Some(val) => format!("config value '{}' does not exist", val),
            None => vec![
                ("api-node", cfg.api_node.as_str()),
                // ("log-path", cfg.log_path.to_str().unwrap()),
            ]
            .iter()
            .map(|(a, b)| format!("{}: {}", a, b))
            .collect::<Vec<_>>()
            .join("\n"),
        };

        println!("{}", msg);
    }
}
