use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, registry, util::SubscriberInitExt};

pub fn setup_tracing() {
    let env_filter = EnvFilter::from_default_env();
    let fmt = fmt::layer().pretty();

    registry().with(env_filter).with(fmt).init()
}
