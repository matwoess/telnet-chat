use std::sync::Arc;

use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream,
    sync::{broadcast, broadcast::Receiver, Mutex},
};

use model::{State, Statement};
use model::CommandType::{ChangeColor, Invalid, Quit};
use model::Statement::{Command, Message};

use crate::error::CommandError;
use crate::util::*;

mod error;
mod model;
mod util;

const SERVER: &str = "localhost";
const PORT: &str = "8001";

#[tokio::main]
async fn main() -> io::Result<()> {
    let address = format!("{}:{}", SERVER, PORT);
    let listener = TcpListener::bind(address).await?;
    let (tx, rx) = broadcast::channel(16);
    let state = Arc::new(Mutex::new(State::new(tx)));
    tokio::spawn(async move {
        server_receiver(rx).await
    });
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Listening to {}", addr);
        let state = state.clone();
        tokio::spawn(async move {
            handle_connection(socket, state).await
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

async fn handle_connection(mut socket: TcpStream<>, state: Arc<Mutex<State>>) -> io::Result<()> {
    write_str_to_socket(&mut socket, "Please enter your name: ").await?;
    let username = match get_from_socket(&mut socket).await {
        Ok(stmt) => match stmt {
            Message(msg) => msg,
            Command(_) => {
                write_str_to_socket(&mut socket, "Names cannot start with '/'\r\n").await?;
                return Ok(());
            }
        },
        Err(e) => {
            println!("error getting username: {}", e);
            return Ok(());
        }
    };
    let mut user = state.lock().await.log_in(&username);
    loop {
        write_to_socket(&mut socket, user.get_prompt()).await?;
        tokio::select! {
        msg = user.rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if !msg.contains(user.get_prompt().as_str()) {
                            write_to_socket(&mut socket, format!("\r{}\r\n", msg)).await?;
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {:?}", e);
                        socket.shutdown().await?;
                        break;
                    }
                }
            }
            stmt = get_from_socket(&mut socket) => {
                match stmt {
                    Ok(stmt) => {
                        match stmt {
                            Message(msg) => {
                                match user.tx.send(user.format_message(msg)) {
                                    Ok(_) => {},
                                    Err(e) => println!("error while sending: {}", e),
                                }
                            },
                            Command(kind) => {
                               match kind {
                                    Quit => {
                                        socket.shutdown().await?;
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
                        socket.shutdown().await?;
                        break;
                    }
                };
            }
        }
    }
    Ok(())
}
