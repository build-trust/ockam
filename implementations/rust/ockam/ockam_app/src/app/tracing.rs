use tracing_subscriber::{filter::LevelFilter, EnvFilter};

pub fn configure_tracing_log() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("OCKAM_LOG")
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init()
}
