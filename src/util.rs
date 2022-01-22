use std::io;

use ansi_term::Color;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::{ChangeColor, Command, CommandError, EmptyStatement, Invalid, Quit};
use crate::model::Statement;

pub(crate) async fn write_to_socket(socket: &mut TcpStream, msg: String) -> io::Result<()> {
    socket.write_all(msg.as_bytes()).await
}

pub(crate) async fn write_str_to_socket(socket: &mut TcpStream, msg: &str) -> io::Result<()> {
    socket.write_all(msg.as_bytes()).await
}

pub(crate) async fn get_from_socket(socket: &mut TcpStream) -> Result<Statement, CommandError> {
    let mut msg = vec![0; 1024];
    loop {
        socket.readable().await?;
        match socket.try_read(&mut msg) {
            Ok(n) => {
                msg.truncate(n);
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    let statement_str = match String::from_utf8(msg) {
        Ok(s) => s.replace("\r\n", ""),
        Err(e) => {
            return Err(CommandError::FromUtf8(e.into()));
        }
    };
    if statement_str.is_empty() {
        return Ok(EmptyStatement);
    }
    let statement = if statement_str.starts_with('/') {
        let args: Vec<&str> = statement_str.split(' ').collect();
        match args[0] {
            "/quit" => Command(Quit),
            "/color" => {
                if args.len() <= 1 {
                    Command(Invalid)
                } else {
                    Command(ChangeColor(String::from(args[1])))
                }
            }
            _ => Command(Invalid)
        }
    } else {
        Statement::Message(statement_str)
    };
    println!("From socket: {:?}", statement);
    Ok(statement)
}

pub(crate) fn get_color_from_string(s: String<>) -> Option<Color> {
    return match s.as_str() {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        "cyan" => Some(Color::Cyan),
        "purple" => Some(Color::Purple),
        _ => None
    };
}