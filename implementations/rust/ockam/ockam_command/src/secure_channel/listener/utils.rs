use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct SecureChannelListenerNodeOpts {
    /// Node
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}
