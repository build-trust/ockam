use crate::CommandGlobalOpts;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct SetCommand {
    /// Name of the configuration value
    pub value: String,
    /// The payload to update the config value with
    pub payload: String,
}

impl SetCommand {
    pub fn run(opts: CommandGlobalOpts, command: SetCommand) {
        let cfg = &opts.config;
        match command.value.as_str() {
            "api-node" => cfg.set_api_node(&command.payload),
            //"log-path" => cfg.set_log_path(&command.payload),
            val => eprintln!("config value '{}' does not exist", val),
        };

        if let Err(e) = cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
        }
    }
}
