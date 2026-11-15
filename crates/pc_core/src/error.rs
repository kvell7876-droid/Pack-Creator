use std::path::PathBuf;

/// Why something the user asked for could not be done. The message is written to be shown as it is.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),

    #[error("{0} is outside the project.")]
    Outside(String),

    #[error("{} could not be read or written: {source}", path.display())]
    Io { path: PathBuf, source: std::io::Error },

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Error::Invalid(message.into())
    }

    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io { path: path.into(), source }
    }
}
