use std::convert::From;
use std::error::Error;
use std::fmt;


#[derive(Clone, Copy, Debug)]
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

impl From<&'static str> for StrError {
    fn from(s: &'static str) -> StrError {
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

impl From<StrError> for StringError {
    fn from(e: StrError) -> StringError {
        From::from(e.description())
    }
}

impl<'a> From<&'a str> for StringError {
    fn from(s: &'a str) -> StringError {
        StringError {
            msg: s.to_owned(),
        }
    }
}

pub type StringResult<T> = Result<T, StringError>;


#[macro_export]
macro_rules! fail {
    ($msg:expr) => {{
            let error = $crate::StrError { msg: $msg };
            return Err(::std::convert::From::from(error));
    }};

    ($msg:expr, $($args:tt)*) => {{
            let error = $crate::StringError { msg: format!($msg, $($args)*) };
            return Err(::std::convert::From::from(error));
    }};
}

#[macro_export]
macro_rules! unwrap {
    ($e:expr, $msg:expr) => { unwrap_or!($e, fail!($msg)) };
    ($e:expr) => {
        unwrap!($e,
                concat!(file!(), ": `", stringify!($e), "` produced `None`"))
    };
}

#[macro_export]
macro_rules! unwrap_or {
    ($e:expr, $or:expr) => {
        match $e {
            Some(x) => x,
            None => $or,
        }
    };

    ($e:expr) => { unwrap_or!($e, return) };
}


