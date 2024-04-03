pub mod tracing;

#[cfg(feature = "log-tower")]
pub mod tower;

#[derive(Debug, thiserror::Error)]
#[error("Could not set tracing logger: {0}")]
pub struct Error(#[from] ::tracing::dispatcher::SetGlobalDefaultError);

pub trait Output: sealed::Output + Send + Sync + 'static {}
mod sealed {
    pub trait Output {
        fn lock() -> impl std::io::Write;
    }

    impl Output for super::Stdout {
        fn lock() -> impl std::io::Write {
            std::io::stdout().lock()
        }
    }

    impl Output for super::Stderr {
        fn lock() -> impl std::io::Write {
            std::io::stderr().lock()
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Stdout;
impl Output for Stdout {}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Stderr;
impl Output for Stderr {}

pub fn setup<O: Output>(level: ::tracing::Level) -> Result<(), Error> {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(tracing::layer::<O>())
        .with(::tracing::level_filters::LevelFilter::from_level(level));

    ::tracing::subscriber::set_global_default(subscriber).map_err(Into::into)
}
