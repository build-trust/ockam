use clap::Args;

#[derive(Clone, Debug, Args)]
pub(crate) struct WithAtNodeOpt {
    #[clap(global = true, long, value_name = "NODE", default_value = "default")]
    pub at: String,
}
