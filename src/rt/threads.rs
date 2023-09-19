#[derive(Debug, thiserror::Error)]
#[error("Expected `auto` or a value in the [1..=255] range")]
pub struct Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Threads {
    Single,
    Auto,
    Multi(u8),
}

impl Threads {
    pub fn parse(input: &str) -> Result<Self, Error> {
        if input == "auto" {
            Ok(Self::Auto)
        } else {
            let count = input.parse().map_err(|_| Error)?;
            if count == 0 {
                Err(Error)
            } else if count == 1 {
                Ok(Self::Single)
            } else {
                Ok(Self::Multi(count))
            }
        }
    }
}
