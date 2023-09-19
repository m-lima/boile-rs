#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "rt-shutdown")]
    #[error(transparent)]
    Shutdown(#[from] crate::rt::shutdown::Error),
    #[cfg(feature = "rt")]
    #[error(transparent)]
    Runtime(#[from] crate::rt::Error),
    #[cfg(feature = "rt")]
    #[error("Could not join task: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Server(#[from] hyper::Error),
}

pub async fn run(
    router: impl Into<Router>,
    addr: impl Into<std::net::SocketAddr>,
) -> Result<(), Error> {
    let start = std::time::Instant::now();

    #[cfg(feature = "log")]
    tracing::info!("Building router");

    let router = match router.into() {
        Router::Simple(router) => router,
        Router::Func(func) => func(),
        Router::Future(future) => future.await,
    };

    #[cfg(feature = "panic")]
    let router = router.layer(crate::panic::CatchPanicLayer::new());

    #[cfg(feature = "log-tower")]
    let router = router.layer(crate::log::tower::layer());

    #[cfg(feature = "log")]
    tracing::info!("Router built");

    let addr = addr.into();

    #[cfg(feature = "log")]
    tracing::info!(%addr, "Binding to address");

    let server = hyper::Server::bind(&addr).serve(router.into_make_service());

    #[cfg(feature = "rt-shutdown")]
    let server = server.with_graceful_shutdown(crate::rt::Shutdown::new()?);

    server.await?;

    tracing::info!(duration = ?start.elapsed(), "Server gracefully shutdown");
    Ok(())
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

#[cfg(feature = "rt")]
#[cfg(feature = "rt-threads")]
pub use inner::start;

#[cfg(feature = "rt")]
#[cfg(not(feature = "rt-threads"))]
pub use inner::start;

#[cfg(feature = "rt")]
mod inner {
    use super::{run, Error, Router};

    #[cfg(feature = "rt-threads")]
    pub fn start(
        router: impl Into<Router>,
        addr: impl Into<std::net::SocketAddr>,
        #[cfg(feature = "rt-threads")] threads: crate::rt::Threads,
    ) -> Result<(), Error> {
        crate::rt::block_on(run(router, addr), threads)?
    }

    #[cfg(not(feature = "rt-threads"))]
    pub fn start(
        router: impl Into<Router>,
        addr: impl Into<std::net::SocketAddr>,
    ) -> Result<(), Error> {
        crate::rt::block_on(run(router, addr))?
    }

    #[cfg(feature = "rt-threads")]
    pub fn start_multiple(
        servers: impl Iterator<Item = (std::net::SocketAddr, Router)>,
        threads: crate::rt::Threads,
    ) -> Result<(), Error> {
        crate::rt::block_on(spawn_servers(servers), threads)?
    }

    #[cfg(feature = "rt")]
    #[cfg(not(feature = "rt-threads"))]
    pub fn start_multiple(
        servers: impl Iterator<Item = (std::net::SocketAddr, Router)>,
    ) -> Result<(), Error> {
        crate::rt::block_on(spawn_servers(servers))?
    }

    async fn spawn_servers(
        servers: impl Iterator<Item = (std::net::SocketAddr, Router)>,
    ) -> Result<(), Error> {
        let mut result = Ok(());
        let servers = servers
            .map(|(addr, router)| tokio::spawn(run(router, addr)))
            .collect::<Vec<_>>();
        for server in servers {
            if let Err(e) = server.await {
                result = Err(Error::TaskJoin(e));
            }
        }
        result
    }
}
