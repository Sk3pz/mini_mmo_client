use better_term::style::Color;
use std::net::TcpStream;
use std::io::{Write, Read, stdin};
use crate::network::entry_point_io::{write_entry_point_ver, write_entry_login_attempt};
use crate::network::entry_response_io::read_entry_response;
use crate::network::login_data::LoginData;
use crate::network::event_io::{read_event, write_event_keepalive, write_event_message, write_event_error};
use std::io;
use std::process::Command;
use crate::command::CommandMuncher;
use crossterm_cursor::cursor;

pub mod utils;
pub mod packet_capnp;
mod network;
mod command;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(target_os = "linux")]
pub const CLEAR: &str = "clear";
#[cfg(target_os = "windows")]
pub const CLEAR: &str = "cls";
#[cfg(target_os = "macos")]
pub const CLEAR: &str = "clear";

fn clear_console() {
    Command::new(CLEAR).status().expect("Failed to run clear command");
}

fn set_cursor_pos(x: u16, y: u16) {
    cursor().goto(x, y);
}

fn connection_err(ip: &str, port: &str) {
    eprintln!("{}Failed to connect to the server.", Color::Red);
}

fn nom_data(data: String) -> Vec<String> {
    let mut cmds = Vec::new();
    let mut current = String::from("");
    let mut flag = false;
    for c in data.chars() {
        match c {
            '!' => flag = true,
            ';' => {
                if flag {
                    flag = false;
                    current = format!("{};", current.clone());
                    continue;
                }

                cmds.push(current.clone());
                current = String::from("");
            }
            _ => {
                if flag {
                    flag = false;
                    current = format!("{}!", current.clone());
                }
                current = format!("{}{}", current.clone(), c);
            }
        }
    }
    cmds.push(current);
    cmds
}

pub fn read_console() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).expect("Error reading from terminal: could not read from input");
    line
}

pub fn get_input<S: Into<String>>(prompt: S) -> String {
    print!("{}", prompt.into());
    io::stdout().flush();
    let read = read_console();
    let input = read.replace("\n", "");
    clear_console();
    input
}

fn main() {

    // TODO: Add server selector and ability to create and remove servers
    let ip = "localhost";
    let port = "2277";
    let address = format!("{}:{}", ip, port);

    let mut stream_result = TcpStream::connect(address.clone());
    if stream_result.is_err() {
        connection_err(ip, port);
        return;
    }

    let mut stream = stream_result.unwrap();

    write_entry_point_ver(&stream, VERSION.to_string());

    let (valid, _, server_version, err) = read_entry_response(&stream);
    if server_version.is_some() {
        //println!("Valid: {}\nserver version: {}", valid, server_version.unwrap());
    } else {
        if err.is_some() {
            println!("Valid: {}\nerror: {}", valid, err.unwrap());
        } else {
            connection_err(ip, port);
        }
        return;
    }
    drop(stream); // stop the ping connection

    loop {

        // establish new connection
        stream_result = TcpStream::connect(address.clone());
        if stream_result.is_err() {
            connection_err(ip, port);
            return;
        }
        stream = stream_result.unwrap();

        let mut signup = false;
        let mut email = String::new();

        let mut su = "";
        let mut read = format!("");
        let mut loop_count: usize = 0;
        // get if the user is signing up
        while su != "y" && su != "n" && su != "yes" && su != "no" {
            if loop_count != 0 {
                println!("Invalid response! Type 'y' for yes and 'n' for no.");
            }
            read = get_input("Are you signing up? (y for yes and n for no): ");
            su = read.as_str();
            loop_count += 1;
        }

        // if the user is signing up, get email
        if su == "y" || su == "yes" {
            signup = true;
            email = get_input("Enter your email: ");
        }

        // get the username
        let username = get_input("Enter your username: ");

        let mut passwd = get_input("Enter your password: ");

        if signup {
            let mut pass_valid = false;

            let mut password_check = get_input("Enter your password again: ");

            if passwd == password_check {
                pass_valid = true;
            }

            // try until the passwords match
            while !pass_valid {
                println!("The passwords did not match!");

                passwd = get_input("Enter your password: ");
                password_check = get_input("Enter your password again: ");

                if passwd == password_check {
                    pass_valid = true;
                }
            }
        }

        let login_data = LoginData {
            email,
            username,
            passwd,
            signup
        };

        // attempt login
        write_entry_login_attempt(&stream, login_data);
        let (login_valid, login_motd, _, login_err) = read_entry_response(&stream);

        // if login was valid, print motd and exit login loop
        if login_valid && login_motd.is_some() {
            println!("Logged in successfully!\n{}", login_motd.unwrap());
            break;
        }
        // login was not valid, print data and retry
        println!("Login Attempt Failed.");
        if login_err.is_some() {
            println!("{}", login_err.unwrap());
        }
        drop(stream);
    }

    let mut muncher: CommandMuncher<Result<(), String>> = CommandMuncher::new();

    muncher.register("clear", |args| {
        clear_console();
        Ok(())
    });
    muncher.register("cursor", |args| {
        if args.len() < 2 {
            return Err("Invalid parameters: cursor command takes two parameters of type u16".to_string());
        }
        let xs = args.get(0).unwrap();
        let ys = args.get(1).unwrap();

        let xr = xs.parse::<u16>();
        let yr = ys.parse::<u16>();
        if xr.is_err() || yr.is_err() {
            return Err("Invalid parameters: cursor command takes two parameters of type u16".to_string());
        }

        let x = xr.unwrap();
        let y = yr.unwrap();

        set_cursor_pos(x, y);
        Ok(())
    });
    muncher.register("print", |args| {
        print!("{}", args.join(" "));
        Ok(())
    });
    muncher.register("println", |args| {
        println!("{}", args.join(" "));
        Ok(())
    });

    // main loop
    loop {
        // read an event
        let (msg, time, error, disconnect_status) = read_event(&stream);

        if msg.is_some() {
            // ======= GAME LOGIC =======

            let server_msg = msg.unwrap();

            // print message from server
            print!("{}", server_msg.message);

            // Process the commands from the server
            for cmd in nom_data(server_msg.data) {
                muncher.munch(cmd);
            }

            // get input and send it to the server to process
            let input = get_input("> ");
            write_event_message(&stream, input, "".to_string());
        } else if time.is_some() {
            write_event_keepalive(&stream);
        } else {
            if error.is_some() {
                println!("{}", error.unwrap());
            } else {
                println!("Invalid packet received from the server.");
            }
        }

        if disconnect_status {
            println!("You have been disconnected.");
            break;
        }
    }

}
