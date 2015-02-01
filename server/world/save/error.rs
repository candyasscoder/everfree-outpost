use std::error;
use std::io;
use std::result;

use util::StrError;

#[derive(Show)]
pub enum Error {
    Io(io::IoError),
    Str(StrError),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Str(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e as &error::Error),
            Error::Str(ref e) => Some(e as &error::Error),
        }
    }
}

impl error::FromError<io::IoError> for Error {
    fn from_error(e: io::IoError) -> Error {
        Error::Io(e)
    }
}

impl error::FromError<StrError> for Error {
    fn from_error(e: StrError) -> Error {
        Error::Str(e)
    }
}

pub type Result<T> = result::Result<T, Error>;
