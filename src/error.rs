use std::fmt::{Display, Formatter};
use std::error::Error;

#[derive(Debug)]
pub enum FtpError {
    InvalidResponse(String),
    UnexpectedReturnCode(i32, String),
    IoError(::std::io::Error)
}

impl Error for FtpError {

    fn description(&self) -> &str {
        match *self {
            FtpError::InvalidResponse(_) => "Server response is in invalid format",
            FtpError::UnexpectedReturnCode(_,_) => "Received unexpected return code.",
            FtpError::IoError(_) => "Comunication IO error",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            FtpError::IoError(ref err) => Some(err),
            _ => None
        }
    }
}

impl Display for FtpError {
    fn fmt(&self, f: &mut Formatter) -> ::std::fmt::Result {
        match *self {
            FtpError::InvalidResponse(ref line) => write!(f, "Server response is in invalid format. Received line: \"{}\".", line),
            FtpError::UnexpectedReturnCode(ref code, ref descr) => write!(f, "Received unexpected return code {}. Description \"{}\".", code, descr),
            FtpError::IoError(ref err) => write!(f, "Comunication error: {}.", err),
        }
    }
}

impl ::std::convert::From<::std::io::Error> for FtpError {
    fn from(err: ::std::io::Error) -> Self {
        FtpError::IoError(err)
    }
}
