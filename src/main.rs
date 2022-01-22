use std::collections::LinkedList;
use std::sync::Arc;

use ansi_term::Color;
use rand::{prelude::IteratorRandom, thread_rng};
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream,
    sync::{broadcast, broadcast::{Receiver, Sender}, Mutex},
};

use CommandType::{ChangeColor, Invalid, Quit};
use Statement::{Command, Message};

use crate::error::CommandError;

mod error;

const SERVER: &str = "localhost";
const PORT: &str = "8001";

const COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Blue,
    Color::Yellow,
    Color::Cyan,
    Color::Purple,
];

#[derive(Debug)]
struct State {
    server_tx: Sender<String>,
    //users: HashMap<String, User>,
    messages: LinkedList<String>,
}

impl State {
    fn new(tx: Sender<String>) -> Self {
        Self { server_tx: tx, /*users: HashMap::new(),*/ messages: LinkedList::new() }
    }

    fn log_in(&mut self, username: &String) -> User {
        let client_tx = self.server_tx.clone();
        let client_rx = self.server_tx.subscribe();
        let user = User::new(username.clone(), client_tx, client_rx);
        //self.users.insert(username, user);
        return user;
    }
}

#[derive(Debug)]
struct User {
    name: String,
    color: Color,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl User {
    fn new(name: String, tx: Sender<String>, rx: Receiver<String>) -> Self {
        let color_idx = (0..COLORS.len()).choose(&mut thread_rng()).unwrap();
        let color = COLORS[color_idx];
        Self { name, color, tx, rx }
    }

    fn get_prompt(&self) -> String {
        self.color.paint(format!("\r[{}]: ", self.name)).to_string()
    }

    fn format_message(&self, msg: String) -> String {
        format!("{}{}", self.get_prompt(), msg)
    }
}

#[derive(Debug)]
enum CommandType {
    Quit,
    ChangeColor(String),
    Invalid,
}

#[derive(Debug)]
enum Statement {
    Command(CommandType),
    Message(String),
}

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

async fn server_receiver(mut receiver: Receiver<String>) {
    loop {
        match receiver.recv().await {
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
                        if !msg.contains(format!("[{}]", user.name).as_str()) {
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
                                        let new_color = match to_color.as_str() {
                                            "red" => Color::Red,
                                            "green" => Color::Green,
                                            "blue" => Color::Blue,
                                            "yellow" => Color::Yellow,
                                            "cyan" => Color::Cyan,
                                            "purple" => Color::Purple,
                                            _ => user.color
                                        };
                                        if new_color == user.color {
                                            write_str_to_socket(&mut socket, "Invalid color change\r\n").await?;
                                        } else {
                                            user.color = new_color;
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

async fn write_to_socket(socket: &mut TcpStream, msg: String) -> io::Result<()> {
    socket.write_all(msg.as_bytes()).await
}

async fn write_str_to_socket(socket: &mut TcpStream, msg: &str) -> io::Result<()> {
    socket.write_all(msg.as_bytes()).await
}

async fn get_from_socket(socket: &mut TcpStream) -> Result<Statement, CommandError> {
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
        return Err(CommandError::new("Empty statement!"));
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
