pub use inner::Shutdown;

#[derive(Debug, thiserror::Error)]
#[error("Could not hook on shutdown signal: {0}")]
pub struct Error(#[from] tokio::io::Error);

#[cfg(unix)]
mod inner {
    pub struct Shutdown(tokio::signal::unix::Signal, tokio::signal::unix::Signal);

    impl Shutdown {
        pub fn new() -> Result<Self, super::Error> {
            Ok(Self(
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?,
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
            ))
        }
    }

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
mod inner {
    pub struct Shutdown(tokio::signal::windows::CtrlC);

    impl Shutdown {
        pub fn new() -> Result<Self, super::Error> {
            Ok(Self(tokio::signal::windows::ctrl_c()?))
        }
    }

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
