#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "log")]
pub mod log;

#[cfg(feature = "rt")]
pub mod rt;

#[cfg(feature = "panic")]
pub mod panic;

#[cfg(any(feature = "server-h1", feature = "server-h2"))]
pub mod server;
