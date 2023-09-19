#[cfg(feature = "rt-shutdown")]
pub mod shutdown;

#[cfg(feature = "rt-shutdown")]
pub use shutdown::Shutdown;

#[cfg(feature = "rt-threads")]
pub mod threads;

#[cfg(feature = "rt-threads")]
pub use threads::Threads;

#[derive(Debug, thiserror::Error)]
#[error("Could not build runtime: {0}")]
pub struct Error(#[from] tokio::io::Error);

#[cfg(feature = "rt-threads")]
#[must_use]
pub fn runtime(threads: Threads) -> tokio::runtime::Builder {
    #[cfg(feature = "log")]
    tracing::info!(?threads, "Building tokio runtime");
    match threads {
        Threads::Single => tokio::runtime::Builder::new_current_thread(),
        Threads::Auto => tokio::runtime::Builder::new_multi_thread(),
        Threads::Multi(t) => {
            let mut rt = tokio::runtime::Builder::new_multi_thread();
            rt.worker_threads(usize::from(t));
            rt
        }
    }
}

#[cfg(not(feature = "rt-threads"))]
#[must_use]
pub fn runtime() -> tokio::runtime::Builder {
    #[cfg(feature = "log")]
    tracing::info!(threads = 1, "Building tokio runtime");
    tokio::runtime::Builder::new_current_thread()
}

#[cfg(feature = "rt-threads")]
#[must_use]
pub fn block_on<F: std::future::Future>(future: F, threads: Threads) -> Result<F::Output, Error> {
    Ok(runtime(threads).enable_all().build()?.block_on(future))
}

#[cfg(not(feature = "rt-threads"))]
#[must_use]
pub fn block_on<F: std::future::Future>(future: F) -> Result<F::Output, Error> {
    Ok(runtime().enable_all().build()?.block_on(future))
}
