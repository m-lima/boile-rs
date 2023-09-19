use crate::log;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not hook on shutdown signal: {0}")]
    Tokio(#[from] tokio::io::Error),
    #[error("Error while serving: {0}")]
    Hyper(#[from] hyper::Error),
}

pub(super) async fn run(router: crate::Router, addr: std::net::SocketAddr) -> Result<(), Error> {
    let start = std::time::Instant::now();

    let router = match router {
        crate::Router::Simple(router) => router,
        crate::Router::Func(func) => func(),
        crate::Router::Future(future) => future.await,
    };

    let app = router
        .layer(tower_http::catch_panic::CatchPanicLayer::new())
        .layer(log::tower::layer());

    tracing::info!(%addr, "Binding to address");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown::hook()?)
        .await?;

    tracing::info!(duration = ?start.elapsed(), "Server gracefully shutdown");
    Ok(())
}

#[cfg(unix)]
mod shutdown {

    pub(super) fn hook() -> Result<Shutdown, super::Error> {
        Ok(Shutdown(
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?,
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
        ))
    }

    pub(super) struct Shutdown(tokio::signal::unix::Signal, tokio::signal::unix::Signal);

    impl std::future::Future for Shutdown {
        type Output = ();

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            ctx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            match self.0.poll_recv(ctx) {
                std::task::Poll::Ready(_) => std::task::Poll::Ready(()),
                std::task::Poll::Pending => match self.1.poll_recv(ctx) {
                    std::task::Poll::Ready(_) => std::task::Poll::Ready(()),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                },
            }
        }
    }
}

#[cfg(windows)]
mod shutdown {
    pub(super) fn new() -> Result<Self, Error> {
        Ok(Self(tokio::signal::windows::ctrl_c()?))
    }

    pub(super) struct Shutdown(tokio::signal::windows::CtrlC);

    impl std::future::Future for Shutdown {
        type Output = ();

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            ctx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            match self.0.poll_recv(ctx) {
                std::task::Poll::Ready(_) => std::task::Poll::Ready(()),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }
}
