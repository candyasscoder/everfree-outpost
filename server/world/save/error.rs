use std::error;
use std::fmt;
use std::io;
use std::result;

use util::{StrError, StringError};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Str(StrError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Str(ref e) => e.fmt(f),
        }
    }
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

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<StrError> for Error {
    fn from(e: StrError) -> Error {
        Error::Str(e)
    }
}

impl From<Error> for StringError {
    fn from(e: Error) -> StringError {
        From::from(error::Error::description(&e))
    }
}

pub type Result<T> = result::Result<T, Error>;
