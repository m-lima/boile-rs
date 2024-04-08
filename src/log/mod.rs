mod output;
pub mod tracing;

#[cfg(feature = "log-tower")]
pub mod tower;

pub use output::{Output, Stderr, Stdout};

#[derive(Debug, thiserror::Error)]
#[error("Could not set tracing logger: {0}")]
pub struct Error(#[from] ::tracing::dispatcher::SetGlobalDefaultError);

pub fn setup<O: Output>(output: O, level: ::tracing::Level) -> Result<(), Error> {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(tracing::layer(output))
        .with(::tracing::level_filters::LevelFilter::from_level(level));

    ::tracing::subscriber::set_global_default(subscriber).map_err(Into::into)
}
