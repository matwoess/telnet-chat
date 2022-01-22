use std::collections::{LinkedList};
use std::sync::Arc;

use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
    net::TcpStream,
    sync::{broadcast, broadcast::{Receiver, Sender}, Mutex},
};

use crate::error::CommandError;

mod error;

const SERVER: &str = "localhost";
const PORT: &str = "8001";

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

    fn log_in(&mut self, username: String) -> User {
        let client_tx = self.server_tx.clone();
        let client_rx = self.server_tx.subscribe();
        let user = User::new(client_tx, client_rx, username.clone());
        //self.users.insert(username, user);
        return user;
    }
}

#[derive(Debug)]
struct User {
    name: String,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl User {
    fn new(tx: Sender<String>, rx: Receiver<String>, name: String) -> Self {
        Self { name, tx, rx }
    }
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
        let msg = receiver.recv().await;
        println!("got message '{}'", msg.unwrap());
    }
}

async fn handle_connection(mut socket: TcpStream<>, state: Arc<Mutex<State>>) -> io::Result<()> {
    send_message(&mut socket, "Please enter your name: ").await?;
    let username = match get_message(&mut socket).await {
        Ok(name) => name,
        Err(e) => {
            println!("error getting username: {}", e);
            return Ok(());
        }
    };
    let mut user = state.lock().await.log_in(format!("{}", username));
    loop {
        send_message(&mut socket, format!("[{}] > ", username).as_str()).await?;
        tokio::select! {
        msg = user.rx.recv() => {
                match msg {
                    Ok(msg) => {
                        send_message(&mut socket, format!("\r[usr]: {}\r\n", msg).as_str()).await?;
                    }
                    Err(e) => {
                        eprintln!("error: {:?}", e)
                    }
                }
            }
            msg = get_message(&mut socket) => {
                match msg {
                    Ok(msg) => {
                        if msg == "quit" {
                            socket.shutdown().await?;
                            break;
                        } else {
                            match user.tx.send(msg) {
                                Ok(_) => {},
                                Err(e) => println!("error while sending: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                    }
                };
            }
        }
    }
    Ok(())
}

async fn send_message(socket: &mut TcpStream, msg: &str) -> io::Result<()> {
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
