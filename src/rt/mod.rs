#[cfg(feature = "rt-shutdown")]
pub mod shutdown;

#[cfg(feature = "rt-shutdown")]
pub use shutdown::Shutdown;

#[cfg(feature = "rt-clap")]
pub mod threads;

#[cfg(feature = "rt-clap")]
pub use threads::Threads;

#[derive(Debug, thiserror::Error)]
#[error("Could not build runtime: {0}")]
pub struct Error(#[from] tokio::io::Error);

#[cfg(feature = "rt-threads")]
#[must_use]
pub fn runtime(threads: Threads) -> tokio::runtime::Builder {
    #[cfg(feature = "log")]
    tracing::info!(%threads, "Building tokio runtime");
    match threads {
        Threads::Single => tokio::runtime::Builder::new_current_thread(),
        Threads::Auto => tokio::runtime::Builder::new_multi_thread(),
        Threads::Multi(threads::Count(count)) => {
            let mut rt = tokio::runtime::Builder::new_multi_thread();
            rt.worker_threads(usize::from(count));
            rt
        }
    }
}

#[cfg(not(feature = "rt-threads"))]
#[must_use]
pub fn runtime() -> tokio::runtime::Builder {
    #[cfg(feature = "log")]
    tracing::info!(threads = %"Single", "Building tokio runtime");
    tokio::runtime::Builder::new_current_thread()
}

#[must_use]
pub fn block_on<F: std::future::Future>(
    future: F,
    #[cfg(feature = "rt-threads")] threads: Threads,
) -> Result<F::Output, Error> {
    Ok(runtime(
        #[cfg(feature = "rt-threads")]
        threads,
    )
    .enable_all()
    .build()?
    .block_on(future))
}
