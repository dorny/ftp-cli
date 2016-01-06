use std::fmt::{Display, Formatter};
use std::error::Error;
use std::convert::From;
use std::io;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum FtpError {
    InvalidResponse(String),
    UnexpectedReturnCode(i32, String),
    IoError(io::Error),
    EncodingError(FromUtf8Error),
    OperationFailed(String),
}

impl Error for FtpError {

    fn description(&self) -> &str {
        match *self {
            FtpError::InvalidResponse(_) => "Server response is in invalid format",
            FtpError::UnexpectedReturnCode(_,_) => "Received unexpected return code.",
            FtpError::IoError(_) => "Comunication IO error",
            FtpError::EncodingError(_) => "Received text has invalid encoding.",
            FtpError::OperationFailed(_) => "Operation failed."
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
            FtpError::EncodingError(ref err) => write!(f, "Received text has invalid encoding. Error: \"{}\".", err),
            FtpError::OperationFailed(ref err) => write!(f, "{}", err)
        }
    }
}

impl From<io::Error> for FtpError {
    fn from(err: io::Error) -> Self {
        FtpError::IoError(err)
    }
}

impl From<FromUtf8Error> for FtpError {
    fn from(err: FromUtf8Error) -> Self {
        FtpError::EncodingError(err)
    }
}
