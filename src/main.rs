use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream,
    sync::broadcast::{self, Receiver, Sender},
};

use model::{
    CommandType::{ChangeColor, Invalid, Quit},
    Statement::{Command, EmptyStatement, Message},
    User,
};

use crate::util::*;

mod model;
mod util;

const SERVER: &str = "localhost";
const PORT: &str = "8001";

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = format!("{}:{}", SERVER, PORT);
    let listener = TcpListener::bind(address).await?;
    let (tx, rx) = broadcast::channel(16);
    tokio::spawn(async move {
        server_receiver(rx).await;
    });
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Listening to {}", addr);
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, tx).await {
                eprintln!("Exited connection with error: {:?}", e);
            }
        });
    }
}

async fn server_receiver(mut rx: Receiver<String>) {
    loop {
        match rx.recv().await {
            Ok(msg) => println!("Broadcast message: '{}'", msg),
            Err(e) => eprintln!("error: {}", e),
        }
    }
}

async fn handle_connection(mut socket: TcpStream<>, tx: Sender<String>) -> io::Result<()> {
    write_str_to_socket(&mut socket, "Please enter your name: ").await?;
    let username = match get_from_socket(&mut socket).await {
        Ok(stmt) => match stmt {
            Message(msg) => msg,
            Command(_) => {
                write_str_to_socket(&mut socket, "Name cannot start with '/'\r\n").await?;
                return Ok(());
            }
            EmptyStatement => {
                write_str_to_socket(&mut socket, "Name cannot be empty\r\n").await?;
                return Ok(());
            }
        },
        Err(e) => {
            println!("error getting username: {}", e);
            return Ok(());
        }
    };
    let mut user = User::new(username, tx);
    match user.tx.send(format!("> {} has joined the chat", user.get_name_prefix())) {
        Ok(_) => {}
        Err(e) => println!("error while sending: {}", e),
    }
    loop {
        write_to_socket(&mut socket, user.get_prompt()).await?;
        tokio::select! {
        msg = user.rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if !msg.contains(user.get_name_prefix().as_str()) {
                            write_to_socket(&mut socket, format!("\r{}\r\n", msg)).await?;
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {:?}", e);
                        break;
                    }
                }
            }
            stmt = get_from_socket(&mut socket) => {
                match stmt {
                    Ok(stmt) => {
                        match stmt {
                            EmptyStatement => println!("empty statement!"),
                            Message(msg) => {
                                match user.tx.send(user.format_message(msg)) {
                                    Ok(_) => {},
                                    Err(e) => println!("error while sending: {}", e),
                                }
                            },
                            Command(kind) => {
                               match kind {
                                    Quit => {
                                        break;
                                    }
                                    ChangeColor(to_color) =>{
                                        match get_color_from_string(to_color) {
                                            None => write_str_to_socket(&mut socket, "Invalid color!\r\n").await?,
                                            Some(col) => user.color = col,
                                        }
                                    }
                                    Invalid => {
                                        write_to_socket(&mut socket, String::from("Invalid command!\r\n")).await?;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                        break;
                    }
                };
            }
        }
    }
    match user.tx.send(format!("> {} has left the chat", user.get_name_prefix())) {
        Ok(_) => {}
        Err(e) => println!("error while sending: {}", e),
    }
    socket.shutdown().await?;
    Ok(())
}
