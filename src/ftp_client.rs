use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Error as IoError};
use std::net::{TcpStream, TcpListener, Ipv4Addr, SocketAddrV4};

use ::commands::*;
use ::error::*;
use ::stream::*;

#[derive(Debug, Copy, Clone)]
pub enum FtpMode {
    Active(SocketAddrV4),
    Passive
}

pub struct FtpClient {
    cmd_stream: BufReader<TcpStream>,
    mode: FtpMode,
}

impl FtpClient {

    /// Connects to FTP server and constructs a new `FtpClient`.
    pub fn connect(server: &str) -> Result<FtpClient, FtpError> {
        match TcpStream::connect(server) {
            Ok(stream) => {
                let mut client = FtpClient {
                    cmd_stream: BufReader::new(stream),
                    mode: FtpMode::Passive,
                };
                // Server should welcome the client.
                match client.read_response() {
                    Ok((status::READY_FOR_NEW_USER,_)) => Ok(client),
                    other => Err(to_error(other))
                }
            }
            Err(err) => Err(FtpError::IoError(err))
        }
    }

    /// Set FTP transfer mode (Active or Passive)
    pub fn set_mode(&mut self, mode: FtpMode) {
        self.mode = mode;
    }

    /// Try to authenticate user on server.
    pub fn login(&mut self, user: &str, password: &str) -> Result<bool, FtpError> {
        try!(self.write_command(FtpCommand::USER(user)));
        match self.read_response() {
            Ok((status::USERNAME_OK_NEED_PASSWORD,_)) => {
                try!(self.write_command(FtpCommand::PASS(password)));
                match self.read_response() {
                    Ok((status::LOGIN_SUCCESSFUL,_)) => Ok(true),
                    Ok((status::NOT_LOGGED_IN,_)) | Ok((status::INVALID_USERNAME_OR_PASSWORD,_)) => Ok(false),
                    other => Err(to_error(other))
                }
            }
            Ok((status::LOGIN_SUCCESSFUL,_)) => Ok(true),
            Ok((status::NOT_LOGGED_IN,_)) | Ok((status::INVALID_USERNAME_OR_PASSWORD,_)) => Ok(false),
            other => Err(to_error(other))
        }
    }

    /// Change remote directory.
    pub fn cd(&mut self, path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::CWD(path);
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::FILE_ACTION_OK, _)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    /// Delete file on server
    pub fn delete (&mut self, path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::DELE(path);
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::FILE_ACTION_OK, _)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    /// Download remote file to current local directory.
    pub fn get(&mut self, remote_path: &str, local_path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::RETR(remote_path);
        let mut stream = try!(self.init_data_transfer(cmd, FtpTransferType::Binary));
        let mut file = try!(File::create(local_path));
        try!(stream.write_all_to(&mut file));
        try!(self.end_data_transfer());
        Ok(())
    }

    /// Make directory on server
    pub fn mkdir(&mut self, path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::MKD(path);
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::PATHNAME_CREATED, _)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    /// List remote directory.
    pub fn list(&mut self, path: &str) -> Result<String, FtpError> {
        let cmd = FtpCommand::LIST(path);
        let mut stream = try!(self.init_data_transfer(cmd, FtpTransferType::Text));
        let mut buf :Vec<u8> = Vec::new();
        try!(stream.read_to_end(&mut buf));
        let text = try!(String::from_utf8(buf));
        try!(self.end_data_transfer());
        Ok(text)
    }

    /// Upload local file to server current directory.
    pub fn put(&mut self, local_path: &str, remote_path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::STOR(remote_path);
        let mut stream = try!(self.init_data_transfer(cmd, FtpTransferType::Binary));
        let mut file = try!(File::open(local_path));
        try!(file.write_all_to(&mut stream));
        try!(self.end_data_transfer());
        Ok(())
    }

    /// Get current working directory on server.
    pub fn pwd(&mut self) -> Result<String, FtpError> {
        let cmd = FtpCommand::PWD;
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::PATHNAME_CREATED, path)) => Ok(path[1..path.len()-1].to_string()),
            other => Err(to_error(other))
        }
    }

    /// Send QUIT command to server and close connection (dropping FtpClient).
    pub fn quit(mut self) {
        let cmd = FtpCommand::QUIT;
        match self.write_command(cmd) {
            _ => { /* ignore any error here */ }
        }
    }

    /// Remove directory
    pub fn rmdir(&mut self, path: &str) -> Result<(), FtpError> {
        let cmd = FtpCommand::RMD(path);
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::FILE_ACTION_OK, _)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    /// Read response code and text (rest of a line)
    fn read_response(&mut self) -> Result<(i32, String), FtpError> {
        let mut line = String::new();
        try!(self.cmd_stream.read_line(&mut line));
        let pos = match line.find(' ') {
            Some(pos) => pos,
            None => return Err(FtpError::InvalidResponse(line))
        };

        let code = match line[0..pos].parse::<i32>() {
            Ok(code) => code,
            Err(_) => return Err(FtpError::InvalidResponse(line))
        };

        let text = line[pos+1..].trim().to_string();
        Ok((code, text))
    }

    /// Init data transfer and returns stream.
    fn init_data_transfer(&mut self, command: FtpCommand, transfer: FtpTransferType) -> Result<TcpStream, FtpError> {
        let cmd = FtpCommand::TYPE(transfer);
        try!(self.write_command(cmd));
        match self.read_response() {
            Ok((status::SUCCESS,_)) => { }
            other => return Err(to_error(other))
        };

        match self.mode {
            FtpMode::Active(addr) => self.init_data_transfer_active(command, addr),
            FtpMode::Passive => self.init_data_transfer_passive(command)
        }
    }

    fn init_data_transfer_active(&mut self, command: FtpCommand, addr: SocketAddrV4) -> Result<TcpStream, FtpError> {
        let listener = try!(TcpListener::bind(addr));
        try!(self.write_command(FtpCommand::PORT(addr)));
        match self.read_response() {
            Ok((status::SUCCESS,_)) => {
                try!(self.write_command(command));
                match self.read_response() {
                    Ok((status::OPEN_DATA_CONNECTION,_)) => {
                        let (stream, _) = try!(listener.accept());
                        Ok(stream)
                    }
                    other => Err(to_error(other))
                }
            }
            other => Err(to_error(other))
        }
    }

    fn init_data_transfer_passive(&mut self, command: FtpCommand) -> Result<TcpStream, FtpError> {
        try!(self.write_command(FtpCommand::PASV));
        match self.read_response() {
            Ok((status::ENTERING_PASSIVE_MODE,line)) => {
                let start_pos = line.rfind('(').unwrap() +1;
                let end_pos = line.rfind(')').unwrap();
                let substr = line[start_pos..end_pos].to_string();
                let nums : Vec<u8> = substr.split(',').map(|x| x.parse::<u8>().unwrap()).collect();
                let ip = Ipv4Addr::new(nums[0],nums[1],nums[2],nums[3]);
                let port = to_ftp_port(nums[4] as u16, nums[5] as u16);
                let addr = SocketAddrV4::new(ip,port);
                try!(self.write_command(command));
                let stream = try!(TcpStream::connect(addr));
                match self.read_response() {
                    Ok((status::OPEN_DATA_CONNECTION,_)) => Ok(stream),
                    other => Err(to_error(other))
                }
            }
            other => Err(to_error(other))
        }
    }

    fn end_data_transfer(&mut self) -> Result<(), FtpError> {
        match self.read_response() {
            Ok((status::CLOSING_DATA_CONNECTION,_)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    fn write_command(&mut self, cmd: FtpCommand) -> Result<(), IoError> {
        let mut stream = self.cmd_stream.get_mut();
        try!(stream.write(cmd.to_string().as_bytes()));
        try!(stream.flush());
        Ok(())
    }
}

fn to_error(result: Result<(i32,String),FtpError>) -> FtpError {
    match result {
        Ok((status::OPERATION_FAILED, text)) => FtpError::OperationFailed(text),
        Ok((code,text)) => FtpError::UnexpectedReturnCode(code, text),
        Err(err) => err
    }
}



fn to_ftp_port(b1: u16, b2: u16) -> u16 {
    b1 *256 + b2
}

mod status {
    pub const OPEN_DATA_CONNECTION : i32 = 150;
    pub const SUCCESS : i32 = 200;
    pub const READY_FOR_NEW_USER : i32 = 220;
    pub const ENTERING_PASSIVE_MODE : i32 = 227;
    pub const CLOSING_DATA_CONNECTION : i32 = 226;
    pub const LOGIN_SUCCESSFUL : i32 = 230;
    pub const FILE_ACTION_OK : i32 = 250;
    pub const PATHNAME_CREATED : i32 = 257;
    pub const USERNAME_OK_NEED_PASSWORD : i32 = 331;
    pub const INVALID_USERNAME_OR_PASSWORD : i32 = 430;
    pub const NOT_LOGGED_IN : i32 = 530;
    pub const OPERATION_FAILED : i32 = 550;
}
