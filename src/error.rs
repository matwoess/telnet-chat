use std::fmt;

#[derive(Debug)]
pub enum CommandError {
    FromUtf8(std::string::FromUtf8Error),
    Io(std::io::Error),
    Generic(String),
}

impl CommandError {
    pub(crate) fn new(msg: &str) -> Self {
        CommandError::Generic(msg.to_string())
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CommandError::*;
        match self {
            FromUtf8(e) => write!(f, "{}", e),
            Io(e) => write!(f, "{}", e),
            Generic(e) => write!(f, "{}", e),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for CommandError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        CommandError::FromUtf8(err)
    }
}