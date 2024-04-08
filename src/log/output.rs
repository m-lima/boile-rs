pub trait Output: Send + Sync + 'static {
    fn lock(&self) -> impl std::io::Write;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Stdout;

impl Output for Stdout {
    fn lock(&self) -> impl std::io::Write {
        std::io::stdout().lock()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Stderr;

impl Output for Stderr {
    fn lock(&self) -> impl std::io::Write {
        std::io::stderr().lock()
    }
}
