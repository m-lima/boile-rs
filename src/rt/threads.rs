#[derive(Debug, thiserror::Error)]
#[error("Expected `auto` or a value in the [1..=255] range")]
pub struct Error;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Threads {
    Single,
    Auto,
    Multi(Count),
}

impl Threads {
    #[must_use]
    pub fn auto() -> Self {
        Self::Auto
    }

    #[must_use]
    pub fn count(count: u8) -> Result<Self, Error> {
        if count == 0 {
            Err(Error)
        } else if count == 1 {
            Ok(Self::Single)
        } else {
            Ok(Self::Multi(Count(count)))
        }
    }
}

impl std::fmt::Display for Threads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Threads::Single => f.write_str("Single"),
            Threads::Auto => f.write_str("Auto"),
            Threads::Multi(count) => write!(f, "Multi({count})"),
        }
    }
}

impl std::fmt::Debug for Threads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub fn parse(input: &str) -> Result<Threads, Error> {
    if input == "auto" {
        Ok(Threads::Auto)
    } else {
        input.parse().map_err(|_| Error).and_then(Threads::count)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Count(pub(super) u8);

macro_rules! impl_fmt {
    ($fmt: ident, $($rest: ident),*) => {
        impl_fmt!($fmt);
        impl_fmt!($($rest),*);
    };

    ($fmt: ident) => {
        impl std::fmt::$fmt for Count {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

impl_fmt!(Display, Debug, Octal, Binary, UpperHex, LowerHex, UpperExp, LowerExp);
