extern crate argparse;
extern crate rpassword;

mod error;
mod ftp_client;

use std::io::Write;
use ftp_client::{FtpClient, FtpMode};
use error::FtpError;
use argparse::{ArgumentParser, Store, StoreOption, StoreConst};
use rpassword::read_password;


#[derive(Debug, Clone)]
struct Settings {
    host: String,
    port: String,
    user: Option<String>,
    password: Option<String>,
    mode: FtpMode,
}

impl Settings {
    fn new() -> Settings {
        Settings {
            host: "localhost".to_string(),
            port: "21".to_string(),
            user: None,
            password: None,
            mode: FtpMode::Active,
        }
    }
}


fn main() {
    let mut settings = Settings::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Primitive FTP client written in rust.");

        ap.refer(&mut settings.host)
            .add_argument("host",Store, "Server hostname");

        ap.refer(&mut settings.port)
            .add_argument("port",Store, "Server port");

        ap.refer(&mut settings.user)
            .add_option(&["-u", "--user"], StoreOption, "Username");

        ap.refer(&mut settings.password)
            .add_option(&["-p", "--password"], StoreOption, "Passwrod");

        ap.refer(&mut settings.mode)
            .add_option(&["-P", "--pasive"], StoreConst(FtpMode::Passive), "Use passive mode for data transfers");

        ap.parse_args_or_exit();
    }

    let server = format!("{}:{}",settings.host, settings.port);

    match FtpClient::connect(server.as_ref()) {
        Ok(mut client) => {
            println!("Connected to server");
            login(&mut client, &mut settings);
            client.set_mode(settings.mode);
            command_loop(&mut client);
        }
        Err(err) => print_err(err)
    }
}

fn login(client: &mut FtpClient, settings: &mut Settings) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut is_logged: bool = false;

    while !is_logged {

        let user = match settings.user {
            Some(ref usr) => usr.to_string(),
            None => {
                let os_user = std::env::var("USER").unwrap_or(String::new());
                print!("User ({}): ", os_user);
                stdout.flush().unwrap();
                let mut line = String::new();
                match stdin.read_line(&mut line) {
                    Err(_) => return,
                    Ok(_) => {
                        match line.trim().is_empty() {
                            true => os_user,
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

        match client.login(user.as_ref(), password.as_ref()) {
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

fn command_loop(client: &mut FtpClient) {
    let stdin = std::io::stdin();
    let mut buf = String::new();

    while stdin.read_line(&mut buf).is_ok() {

        {
            let line = buf.trim();
            let (cmd,args) = match line.find(' ') {
                Some(pos) => (line[0..pos].as_ref(), line[pos+1..].as_ref()),
                None => (line, "".as_ref())
            };

            match cmd {
                "ls" => print_result(client.list(args)),
                "cd" => {
                    match client.cd(args) {
                        Ok(_) => {},
                        Err(e) => print_err(e)
                    }
                }
                "pwd" => print_result(client.pwd()),
                "get" => {
                    match client.get(args,args) {
                        Ok(_) => println!("File download complete."),
                        Err(e) => print_err(e)
                    }
                },
                "put" => {
                    match client.put(args,args) {
                        Ok(_) => println!("File upload complete."),
                        Err(e) => print_err(e)
                    }
                },
                "q" => return,
                "" => {},
                _ => println!("Unknown command.")
            }
        }

        buf.clear();
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
