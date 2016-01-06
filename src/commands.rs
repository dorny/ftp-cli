use std::net::{SocketAddrV4};

pub enum FtpCommand<'a> {
    CWD(&'a str),
    LIST(&'a str),
    USER(&'a str),
    PASS(&'a str),
    PASV,
    PORT(SocketAddrV4),
    PWD,
    RETR(&'a str),
    STOR(&'a str),
}

impl<'a> ToString for FtpCommand<'a> {
    fn to_string(&self) -> String {
        match *self {
            FtpCommand::CWD(ref path) => format!("CWD {}\n", path),
            FtpCommand::LIST(ref path) => format!("LIST {}\n", path),
            FtpCommand::USER(ref user) => format!("USER {}\n", user),
            FtpCommand::PASS(ref pass) => format!("PASS {}\n", pass),
            FtpCommand::PASV => format!("PASV\n"),
            FtpCommand::PORT(addr) => {
                let ip = addr.ip().octets();
                let port = addr.port();
                format!("PORT {},{},{},{},{},{}\n", ip[0], ip[1], ip[2], ip[3], port/256, port%256)
            }
            FtpCommand::PWD => format!("PWD\n"),
            FtpCommand::RETR(ref filename) => format!("RETR {}\n", filename),
            FtpCommand::STOR(ref filename) => format!("STOR {}\n", filename),
        }
    }
}
