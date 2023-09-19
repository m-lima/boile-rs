#[cfg(feature = "export")]
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
    #[error("Could not join task: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Server(#[from] server::Error),
}

impl From<Error> for std::process::ExitCode {
    fn from(error: Error) -> Self {
        match error {
            #[cfg(feature = "log")]
            Error::Tracing(_) => std::process::ExitCode::from(1),
            Error::Tokio(_) => std::process::ExitCode::from(2),
            Error::TaskJoin(_) => std::process::ExitCode::from(3),
            Error::Server(server::Error::Tokio(_)) => std::process::ExitCode::from(4),
            Error::Server(server::Error::Hyper(_)) => std::process::ExitCode::from(5),
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
    router: impl Into<Router>,
    addr: impl Into<std::net::SocketAddr>,
    #[cfg(feature = "threads")] threads: Threads,
) -> Result<(), Error> {
    #[cfg(not(feature = "threads"))]
    let threads = Threads::Single;

    runtime(threads)
        .enable_all()
        .build()?
        .block_on(server::run(router.into(), addr.into()))
        .map_err(Error::from)
}

pub fn serve_multiple(
    servers: impl Iterator<Item = (std::net::SocketAddr, Router)>,
    #[cfg(feature = "threads")] threads: Threads,
) -> Result<(), Error> {
    #[cfg(not(feature = "threads"))]
    let threads = Threads::Single;

    runtime(threads)
        .enable_all()
        .build()?
        .block_on(async {
            let mut result = Ok(());
            let servers = servers
                .map(|(addr, router)| tokio::spawn(server::run(router, addr)))
                .collect::<Vec<_>>();
            for server in servers {
                if let Err(e) = server.await {
                    result = Err(e);
                }
            }
            result
        })
        .map_err(Error::from)
}

pub enum Router {
    Simple(axum::Router),
    Func(Box<dyn FnOnce() -> axum::Router + Send>),
    Future(std::pin::Pin<Box<dyn std::future::Future<Output = axum::Router> + Send>>),
}

impl Router {
    #[must_use]
    pub fn simple(router: axum::Router) -> Self {
        Self::Simple(router)
    }

    #[must_use]
    pub fn func<F: FnOnce() -> axum::Router + Send + 'static>(func: F) -> Self {
        Self::Func(Box::new(func))
    }

    #[must_use]
    pub fn future<F: std::future::Future<Output = axum::Router> + Send + 'static>(
        future: F,
    ) -> Self {
        Self::Future(Box::pin(future))
    }
}

impl From<axum::Router> for Router {
    fn from(value: axum::Router) -> Self {
        Self::simple(value)
    }
}

impl<F: FnOnce() -> axum::Router + Send + 'static> From<F> for Router {
    fn from(value: F) -> Self {
        Self::func(value)
    }
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
