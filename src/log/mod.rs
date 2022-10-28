mod layer;
pub(super) mod tower;

#[cfg(not(feature = "log"))]
pub use layer::Layer;

#[cfg(feature = "log")]
pub fn setup(level: tracing::Level) -> Result<(), super::Error> {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(layer::Layer::new())
        .with(tracing::level_filters::LevelFilter::from_level(level));

    tracing::subscriber::set_global_default(subscriber).map_err(super::Error::from)
}
