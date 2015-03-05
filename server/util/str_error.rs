use std::error::{Error, FromError};
use std::fmt;


#[derive(Copy, Debug)]
pub struct StrError {
    pub msg: &'static str,
}

impl Error for StrError {
    fn description(&self) -> &'static str {
        self.msg
    }
}

impl fmt::Display for StrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl FromError<&'static str> for StrError {
    fn from_error(s: &'static str) -> StrError {
        StrError {
            msg: s,
        }
    }
}

pub type StrResult<T> = Result<T, StrError>;

#[derive(Debug)]
pub struct StringError {
    pub msg: String,
}

impl Error for StringError {
    fn description(&self) -> &str {
        &*self.msg
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl FromError<StrError> for StringError {
    fn from_error(e: StrError) -> StringError {
        FromError::from_error(e.description())
    }
}

impl<'a> FromError<&'a str> for StringError {
    fn from_error(s: &'a str) -> StringError {
        StringError {
            msg: String::from_str(s),
        }
    }
}

pub type StringResult<T> = Result<T, StringError>;


macro_rules! fail {
    ($msg:expr) => {{
            let error = $crate::util::StrError { msg: $msg };
            return Err(::std::error::FromError::from_error(error));
    }};

    ($msg:expr, $($args:tt)*) => {{
            let error = $crate::util::StringError { msg: format!($msg, $($args)*) };
            return Err(::std::error::FromError::from_error(error));
    }};
}

macro_rules! unwrap {
    ($e:expr, $msg:expr) => { unwrap_or!($e, fail!($msg)) };
    ($e:expr) => {
        unwrap!($e,
                concat!(file!(), ":", stringify!(line!()),
                ": `", stringify!($e), "` produced `None`"))
    };
}

macro_rules! unwrap_or {
    ($e:expr, $or:expr) => {
        match $e {
            Some(x) => x,
            None => $or,
        }
    };

    ($e:expr) => { unwrap_or!($e, return) };
}


