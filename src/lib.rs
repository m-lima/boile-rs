pub mod log;
mod server;

#[cfg(not(feature = "threads"))]
use threads::Threads;
#[cfg(feature = "threads")]
pub use threads::Threads;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not build runtime: {0}")]
    Tokio(#[from] tokio::io::Error),
    #[error(transparent)]
    Server(#[from] server::Error),
}

#[tracing::instrument]
fn runtime(threads: Threads) -> tokio::runtime::Builder {
    tracing::info!("Building tokio runtime");
    match threads {
        #[cfg(feature = "threads")]
        Threads::Auto => tokio::runtime::Builder::new_multi_thread(),
        Threads::Single => tokio::runtime::Builder::new_current_thread(),
        #[cfg(feature = "threads")]
        Threads::Multi(t) => {
            let mut rt = tokio::runtime::Builder::new_multi_thread();
            rt.worker_threads(usize::from(t));
            rt
        }
    }
}

pub fn serve(
    router: axum::Router,
    addr: std::net::SocketAddr,
    #[cfg(feature = "threads")] threads: Threads,
) -> Result<(), Error> {
    #[cfg(not(feature = "threads"))]
    let threads = Threads::Single;

    runtime(threads)
        .enable_all()
        .build()?
        .block_on(server::run(router, addr))
        .map_err(Error::from)
}

mod threads {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum Threads {
        #[cfg(feature = "threads")]
        Auto,
        Single,
        #[cfg(feature = "threads")]
        Multi(u8),
    }
}
