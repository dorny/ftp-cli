extern crate argparse;
extern crate rpassword;

mod error;
mod ftp_client;
mod stream;
mod commands;

use std::io::Write;
use std::net::SocketAddr;
use std::str::FromStr;
use ftp_client::{FtpClient, FtpMode};
use error::FtpError;
use argparse::{ArgumentParser, Print, Store, StoreOption};
use rpassword::read_password;


#[derive(Debug, Clone)]
struct Settings {
    host: String,
    port: String,
    user: Option<String>,
    password: Option<String>,
    listen: Option<String>,
}

impl Settings {
    fn new() -> Settings {
        Settings {
            host: "localhost".to_string(),
            port: "21".to_string(),
            user: None,
            password: None,
            listen: None,
        }
    }
}


fn main() {
    let mut settings = Settings::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Primitive FTP client written in rust.");

        ap.add_option(&["--version"],
            Print(env!("CARGO_PKG_VERSION").to_string()), "Show version");

        ap.refer(&mut settings.host)
            .add_argument("host",Store, "Server hostname");

        ap.refer(&mut settings.port)
            .add_argument("port",Store, "Server port");

        ap.refer(&mut settings.user)
            .add_option(&["-u", "--user"], StoreOption, "Username");

        ap.refer(&mut settings.password)
            .add_option(&["-p", "--password"], StoreOption, "Passwrod");

        ap.refer(&mut settings.listen)
            .add_option(&["--active"], StoreOption, "Use active mode and listen on provided address for data transfers");

        ap.parse_args_or_exit();
    }

    let server = format!("{}:{}",settings.host, settings.port);

    match FtpClient::connect(&server) {
        Ok(mut client) => {
            println!("Connected to server");
            login(&mut client, &settings);
            set_tranfer_mode(&mut client, &settings);
            command_loop(&mut client);
            client.quit();
        }
        Err(err) => print_err(err)
    }
}

fn login(client: &mut FtpClient, settings: &Settings) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut is_logged: bool = false;
    let os_user = std::env::var("USER").unwrap_or(String::new());

    while !is_logged {

        let user = match settings.user {
            Some(ref usr) => usr.to_string(),
            None => {
                print!("User ({}): ", os_user);
                stdout.flush().unwrap();
                let mut line = String::new();
                match stdin.read_line(&mut line) {
                    Err(_) => return,
                    Ok(_) => {
                        match line.trim().is_empty() {
                            true => os_user.to_string(),
                            false => line.trim().to_string()
                        }
                    }
                }
            }
        };

        let password = match settings.password {
            Some(ref pwd) => pwd.to_string(),
            None => {
                print!("Password: ");
                stdout.flush().unwrap();
                match read_password() {
                    Ok(pwd) => pwd.trim().to_string(),
                    Err(_) => return,
                }
            }
        };

        match client.login(&user, &password) {
            Ok(true) => {
                println!("Successfuly logged in.");
                is_logged = true;
            }
            Ok(false) => {
                println!("Invalid username or password.");
                continue;
            }
            Err(err) => {
                print_err(err);
                return;
            }
        }
    }
}

fn set_tranfer_mode(client: &mut FtpClient, settings: &Settings) {
    if let Some(ref text) = settings.listen {
        match SocketAddr::from_str(text) {
            Ok(SocketAddr::V4(addr)) => client.set_mode(FtpMode::Active(addr)),
            Ok(SocketAddr::V6(_)) => println!("IPv6 for active mode is not supported. Using default passive mode."),
            Err(e) => println!("Invalid listen address format: {}", e)
        }
    }
}

fn command_loop(client: &mut FtpClient) {
    let stdin = std::io::stdin();
    let mut buf = String::new();

    while stdin.read_line(&mut buf).is_ok() {

        {
            let line = buf.trim();
            let (cmd,args) = match line.find(' ') {
                Some(pos) => (&line[0..pos], &line[pos+1..]),
                None => (line, "".as_ref())
            };

            match cmd {

                "cd" => print_if_error(client.cd(args)),

                "get" => {
                    match client.get(args,args) {
                        Ok(_) => println!("File download complete."),
                        Err(e) => print_err(e)
                    }
                }

                "mkdir" => print_if_error(client.mkdir(args)),

                "ls" => print_result(client.list(args)),

                "put" => {
                    match client.put(args,args) {
                        Ok(_) => println!("File upload complete."),
                        Err(e) => print_err(e)
                    }
                }

                "pwd" => print_result(client.pwd()),

                "rm" => print_if_error(client.delete(args)),

                "rmdir" => print_if_error(client.rmdir(args)),

                "q" => return,

                "" => { }

                _ => println!("Unknown command.")
            }
        }

        buf.clear();
    }
}

fn print_if_error(result: Result<(), FtpError>) {
    match result {
        Ok(()) => { }
        Err(e) => println!("{}", e)
    }
}

fn print_result(result: Result<String, FtpError>) {
    match result {
        Ok(text) => println!("{}", text),
        Err(e) => println!("{}", e)
    }
}


fn print_err(error: FtpError) {
    println!("{}", error);
}
