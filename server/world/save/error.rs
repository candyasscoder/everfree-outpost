use std::error;
use std::fmt;
use std::old_io;
use std::result;

use util::{StrError, StringError};

#[derive(Debug)]
pub enum Error {
    Io(old_io::IoError),
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

impl error::FromError<old_io::IoError> for Error {
    fn from_error(e: old_io::IoError) -> Error {
        Error::Io(e)
    }
}

impl error::FromError<StrError> for Error {
    fn from_error(e: StrError) -> Error {
        Error::Str(e)
    }
}

impl error::FromError<Error> for StringError {
    fn from_error(e: Error) -> StringError {
        error::FromError::from_error(error::Error::description(&e))
    }
}

pub type Result<T> = result::Result<T, Error>;
