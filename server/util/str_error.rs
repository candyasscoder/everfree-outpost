use std::error::Error;
use std::fmt;


#[derive(Copy, Show)]
pub struct StrError {
    pub msg: &'static str,
}

impl Error for StrError {
    fn description(&self) -> &'static str {
        self.msg
    }
}

impl fmt::String for StrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::String::fmt(&self.msg, f)
    }
}

macro_rules! fail {
    ($msg:expr) => {{
            let error = $crate::util::StrError { msg: $msg };
            return Err(::std::error::FromError::from_error(error));
    }};
}

macro_rules! unwrap {
    ($e:expr, $msg:expr) => { unwrap_or!($e, fail!($msg)) };
    ($e:expr) => {
        unwrap!($e,
                concat!(file!(), ":", stringify2!(line!()),
                ": `", stringify!($e), "` produced `None`"))
    };
}

macro_rules! stringify2 {
    ($e:expr) => { stringify!($e) };
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

pub type StrResult<T> = Result<T, StrError>;
