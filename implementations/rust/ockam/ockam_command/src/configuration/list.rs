use crate::CommandGlobalOpts;
use clap::Args;
use ockam_api::config::lookup::LookupValue;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let lookup = options.config.lookup();

        for (alias, value) in &lookup.map {
            // Currently we only have this one type of lookup but we
            // need to be ready for more values.  Remove this "allow"
            // in the future
            #[allow(irrefutable_let_patterns)]
            if let LookupValue::Address(addr) = value {
                println!("Node:    {}\nAddress: {}\n", alias, addr);
            }
        }
    }
}
