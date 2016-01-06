use std::net::{SocketAddrV4};

pub enum FtpCommand<'a> {
    CWD(&'a str),
    DELE(&'a str),
    LIST(&'a str),
    MKD(&'a str),
    PASS(&'a str),
    PASV,
    PORT(SocketAddrV4),
    PWD,
    QUIT,
    RETR(&'a str),
    RMD(&'a str),
    STOR(&'a str),
    USER(&'a str),
}

impl<'a> ToString for FtpCommand<'a> {
    fn to_string(&self) -> String {
        match *self {
            FtpCommand::CWD(ref path) => format!("CWD {}\n", path),
            FtpCommand::DELE(ref path) => format!("DELE {}\n", path),
            FtpCommand::LIST(ref path) => format!("LIST {}\n", path),
            FtpCommand::MKD(ref path) => format!("MKD {}\n", path),
            FtpCommand::PASS(ref pass) => format!("PASS {}\n", pass),
            FtpCommand::PASV => format!("PASV\n"),
            FtpCommand::PORT(addr) => {
                let ip = addr.ip().octets();
                let port = addr.port();
                format!("PORT {},{},{},{},{},{}\n", ip[0], ip[1], ip[2], ip[3], port/256, port%256)
            }
            FtpCommand::PWD => format!("PWD\n"),
            FtpCommand::QUIT => format!("QUIT\n"),
            FtpCommand::RETR(ref path) => format!("RETR {}\n", path),
            FtpCommand::RMD(ref path) => format!("RMD {}\n", path),
            FtpCommand::STOR(ref path) => format!("STOR {}\n", path),
            FtpCommand::USER(ref user) => format!("USER {}\n", user),
        }
    }
}
