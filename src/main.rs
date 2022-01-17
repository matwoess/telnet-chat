use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
};
use tokio::net::TcpStream;

use crate::error::CommandError;

mod error;

const SERVER: &str = "localhost";
const PORT: &str = "8001";

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = format!("{}:{}", SERVER, PORT);
    let listener = TcpListener::bind(address).await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Listening to {}", addr);
        tokio::spawn(async move {
            handle_connection(socket).await
        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> io::Result<()> {
    let mut username = String::from("");
    loop {
        if username.is_empty() {
            send_msg(&mut socket, "Please enter your name: ").await?;
        } else {
            send_msg(&mut socket, format!("[{}] > ", username).as_str()).await?;
        }
        let result = get_message(&mut socket).await;
        match result {
            Ok(msg) => {
                if msg == "quit" {
                    socket.shutdown().await?;
                    break;
                } else {
                    if username.is_empty() {
                        username = msg;
                    } else {
                        add_chat_message()
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
            }
        };
    }
    Ok(())
}

fn add_chat_message() {
    todo!()
}

async fn send_msg(socket: &mut TcpStream, msg: &str) -> io::Result<()> {
    socket.write_all(msg.as_bytes()).await
}

async fn get_message(socket: &mut TcpStream) -> Result<String, CommandError> {
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
    let str_msg = match String::from_utf8(msg) {
        Ok(s) => s.replace("\r\n", ""),
        Err(e) => {
            return Err(CommandError::FromUtf8(e.into()));
        }
    };
    if str_msg.is_empty() {
        return Err(CommandError::new("Empty statement!"));
    }
    println!("Statement = {:?}", str_msg);
    Ok(str_msg)
}
