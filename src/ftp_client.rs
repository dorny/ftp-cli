use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader};
use std::io::Error as IoError;
use std::net::{TcpStream, TcpListener, Ipv4Addr, SocketAddrV4};

use ::error::*;

#[derive(Debug, Copy, Clone)]
pub enum FtpMode {
    Active,
    Passive
}

pub struct FtpClient {
    cmd_stream: BufReader<TcpStream>,
    mode: FtpMode,
}

impl FtpClient {

    pub fn connect(server: &str) -> Result<FtpClient, FtpError> {
        match TcpStream::connect(server) {
            Ok(stream) => {
                let mut client = FtpClient {
                    cmd_stream: BufReader::new(stream),
                    mode: FtpMode::Passive,
                };
                client.init().map(|_| client)
            }
            Err(err) => Err(FtpError::IoError(err))
        }
    }

    fn init(&mut self) -> Result<(), FtpError> {
        match self.read_response() {
            Ok((status::READY_FOR_NEW_USER,_)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    pub fn set_mode(&mut self, mode: FtpMode) {
        self.mode = mode;
    }

    pub fn login(&mut self, user: &str, password: &str) -> Result<bool, FtpError> {
        self.write_command_with_param(commands::USER, user);
        match self.read_response() {
            Ok((status::USERNAME_OK_NEED_PASSWORD,_)) => {
                self.write_command_with_param(commands::PASS, password);
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

    pub fn list(&mut self, arg: &str) -> Result<String, FtpError> {
        let cmd = format!("{} {}\n",commands::LIST, arg);
        let mut stream = try!(self.data_request(cmd.as_ref()));
        let data = try!(self.receive_data(&mut stream));
        let text = String::from_utf8(data).unwrap();
        Ok(text)
    }

    pub fn cd(&mut self, path: &str) -> Result<(), FtpError> {
        self.write_command_with_param(commands::CWD, path);
        match self.read_response() {
            Ok((status::FILE_ACTION_OK, _)) => Ok(()),
            other => Err(to_error(other))
        }
    }

    pub fn pwd(&mut self) -> Result<String, FtpError> {
        self.write_command(commands::PWD);
        match self.read_response() {
            Ok((status::PATHNAME_CREATED, path)) => Ok(path[1..path.len()-1].to_string()),
            other => Err(to_error(other))
        }
    }

    pub fn get(&mut self, remote: &str, local: &str) -> Result<(), FtpError> {
        let cmd = format!("{} {}\n",commands::RETR, remote);
        let stream = try!(self.data_request(cmd.as_ref()));
        let file = try!(File::create(local));
        self.send_data_to(stream,file)
            .map_err(|e| FtpError::IoError(e))
    }

    pub fn put(&mut self, local: &str, remote: &str) -> Result<(), FtpError> {
        let cmd = format!("{} {}\n",commands::STOR, remote);
        let stream = try!(self.data_request(cmd.as_ref()));
        let file = try!(File::open(local));
        self.send_data_to(file,stream)
            .map_err(|e| FtpError::IoError(e))
    }

    fn read_response(&mut self) -> Result<(i32, String), FtpError> {
        let mut line = String::new();
        try!(self.cmd_stream.read_line(&mut line));
        //println!("received response: {:?}", line);
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

    fn data_request(&mut self, command: &str) -> Result<TcpStream, FtpError> {
        match self.mode {
            FtpMode::Active => self.data_request_active(command),
            FtpMode::Passive => self.data_request_passive(command)
        }
    }

    fn data_request_active(&mut self, command: &str) -> Result<TcpStream, FtpError> {
        let (listener, addr) = try!(self.create_listener());
        self.write_command_with_param(commands::PORT, to_ftp_addr(addr).as_ref());
        match self.read_response() {
            Ok((status::SUCCESS,_)) => {
                self.write_command(command);
                match self.read_response() {
                    Ok((status::OPEN_DATA_CONNECTION,_)) => {
                        let (stream, _) = try!(listener.accept());
                        Ok(stream)
                    },
                    other => Err(to_error(other))
                }
            },
            other => Err(to_error(other))
        }
    }

    fn data_request_passive(&mut self, command: &str) -> Result<TcpStream, FtpError> {
        self.write_command(commands::PASV);
        match self.read_response() {
            Ok((status::ENTERING_PASSIVE_MODE,line)) => {
                let start_pos = line.rfind('(').unwrap() +1;
                let end_pos = line.rfind(')').unwrap();
                let substr = line[start_pos..end_pos].to_string();
                println!("substr: {:?}", substr);
                let nums : Vec<u8> = substr.split(',').map(|x| x.parse::<u8>().unwrap()).collect();
                let ip = Ipv4Addr::new(nums[0],nums[1],nums[2],nums[3]);
                let port = to_ftp_port(nums[4] as u16, nums[5] as u16);
                let addr = SocketAddrV4::new(ip,port);
                println!("addr: {:?}", addr);
                self.write_command(command);
                let stream = TcpStream::connect(addr).map_err(|e| FtpError::IoError(e));
                match self.read_response() {
                    Ok((status::OPEN_DATA_CONNECTION,_)) => stream,
                    other => Err(to_error(other))
                }
            },
            other => Err(to_error(other))
        }
    }

    fn write_command(&mut self, cmd: &str) {
        //println!("sending command: {:?}", cmd);
        let mut stream = self.cmd_stream.get_mut();
        stream.write(cmd.as_bytes()).unwrap();
        stream.write("\n".as_bytes()).unwrap();
        stream.flush().unwrap();
    }

    fn write_command_with_param(&mut self, cmd: &str, param: &str) {
        self.write_command(format!("{} {}",cmd, param).as_ref());
    }

    fn receive_data(&mut self, stream: &mut TcpStream) -> Result<Vec<u8>, FtpError> {
        let mut buf :Vec<u8> = Vec::new();
        try!(stream.read_to_end(&mut buf));
        match self.read_response() {
            Ok((status::CLOSING_DATA_CONNECTION,_)) => Ok(buf),
            other => Err(to_error(other))
        }
    }

    fn send_data_to<R: Read, W: Write>(&mut self, istream: R, mut ostream: W) -> Result<(), IoError> {
        let mut reader = BufReader::new(istream);
        let mut done = false;

        while !done {
            let count;
            {
                let buf = try!(reader.fill_buf());
                count = buf.len();
                if count > 0 {
                    try!(ostream.write_all(buf));
                }
                else {
                    done = true;
                }
            }

            reader.consume(count);
        }

        Ok(())
    }

    fn create_listener(&mut self) -> Result<(TcpListener, SocketAddrV4), IoError> {
        let localhost = Ipv4Addr::new(127,0,0,1);
        let mut port = try!(self.cmd_stream.get_ref().local_addr()).port()+1;
        let mut num = 1;

        while num < 10 {
            let addr = SocketAddrV4::new(localhost, port);
            match TcpListener::bind(SocketAddrV4::new(localhost, port)) {
                Ok(listener) => return Ok((listener, addr)),
                Err(_) => {
                    num += 1;
                    port += 1;
                }
            }
        }

        let addr = SocketAddrV4::new(localhost, port);
        TcpListener::bind(addr).map(|l| (l, addr))
    }
}

fn to_error(result: Result<(i32,String),FtpError>) -> FtpError {
    match result {
        Ok((code,text)) => FtpError::UnexpectedReturnCode(code, text),
        Err(err) => err
    }
}

fn to_ftp_addr(addr: SocketAddrV4) -> String {
    let ip = addr.ip().octets();
    let port = addr.port();
    format!("{},{},{},{},{},{}", ip[0], ip[1], ip[2], ip[3], port/256, port%256)
}

fn to_ftp_port(b1: u16, b2: u16) -> u16 {
    b1 *256 + b2
}

mod commands {
    pub static CWD : &'static str = "CWD";
    pub static LIST : &'static str = "LIST";
    pub static USER : &'static str = "USER";
    pub static PASS : &'static str = "PASS";
    pub static PASV : &'static str = "PASV";
    pub static PORT : &'static str = "PORT";
    pub static PWD : &'static str = "PWD";
    pub static RETR : &'static str = "RETR";
    pub static STOR : &'static str = "STOR";
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
}
