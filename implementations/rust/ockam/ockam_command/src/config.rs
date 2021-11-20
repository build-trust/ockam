use crate::command::{CommandResult};
use crate::AppError;

pub struct AppConfig {}

/* WIP
const CONFIG_ARG: &str = "config";
const SECRETS_ARG: &str = "secrets";
const OCKAM_ENV_PREFIX: &str = "OCKAM";
*/

impl AppConfig {
    pub fn evaluate() -> Result<CommandResult, AppError> {
        Err(AppError::Unknown)

    /*
        let mut config = config::Config::default();
        let yaml = load_yaml!("cli.yml");
        let args = App::from_yaml(yaml).get_matches();

        let config_file = args.value_of(CONFIG_ARG);

        if let Some(config_file) = config_file {
            if config.merge(config::File::with_name(config_file)).is_ok() {
                debug!("Loaded settings from {} config file", config_file)
            } else {
                warn!("Unable to load settings from {}", config_file)
            }
        } else {
            warn!("No config file specified.")
        }

        let secrets_file = args.value_of(SECRETS_ARG);

        if let Some(secrets_file) = secrets_file {
            if config.merge(config::File::with_name(secrets_file)).is_ok() {
                debug!("Loaded secrets from {} secrets file.", secrets_file)
            } else {
                warn!("Unable to load secrets from {}", secrets_file)
            }
        } else {
            debug!("No secrets file specified.")
        }

        config
            .merge(config::Environment::with_prefix(OCKAM_ENV_PREFIX))
            .ok();

        let (command_name, command_args) = args.subcommand();
        let mut command: Command = command_name.parse()?;
        command.run(command_args)
    */
    }
}
