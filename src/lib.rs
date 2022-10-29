pub use {axum, hyper, tokio, tower_http, tracing, tracing_subscriber};

pub mod log;
mod server;

#[cfg(not(feature = "threads"))]
use threads::Threads;
#[cfg(feature = "threads")]
pub use threads::Threads;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "log")]
    #[error("Could not set tracing logger: {0}")]
    Tracing(#[from] tracing::dispatcher::SetGlobalDefaultError),
    #[error("Could not build runtime: {0}")]
    Tokio(#[from] tokio::io::Error),
    #[error(transparent)]
    Server(#[from] server::Error),
}

impl From<Error> for std::process::ExitCode {
    fn from(error: Error) -> Self {
        match error {
            #[cfg(feature = "log")]
            Error::Tracing(_) => std::process::ExitCode::from(1),
            Error::Tokio(_) => std::process::ExitCode::from(2),
            Error::Server(server::Error::Tokio(_)) => std::process::ExitCode::from(3),
            Error::Server(server::Error::Hyper(_)) => std::process::ExitCode::from(4),
        }
    }
}

impl std::process::Termination for Error {
    fn report(self) -> std::process::ExitCode {
        self.into()
    }
}

fn runtime(threads: Threads) -> tokio::runtime::Builder {
    tracing::info!(?threads, "Building tokio runtime");
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
    addr: impl Into<std::net::SocketAddr>,
    #[cfg(feature = "threads")] threads: Threads,
) -> Result<(), Error> {
    #[cfg(not(feature = "threads"))]
    let threads = Threads::Single;

    runtime(threads)
        .enable_all()
        .build()?
        .block_on(server::run(router, addr.into()))
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
