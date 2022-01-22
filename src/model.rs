use tokio::sync::broadcast::{Receiver, Sender};
use std::collections::LinkedList;
use ansi_term::Colour as Color;
use rand::thread_rng;
use rand::prelude::IteratorRandom;

#[derive(Debug)]
pub(crate) struct State {
    server_tx: Sender<String>,
    //users: HashMap<String, User>,
    messages: LinkedList<String>,
}

impl State {
    pub(crate) fn new(tx: Sender<String>) -> Self {
        Self { server_tx: tx, /*users: HashMap::new(),*/ messages: LinkedList::new() }
    }

    pub(crate) fn log_in(&mut self, username: &String) -> User {
        let client_tx = self.server_tx.clone();
        let client_rx = self.server_tx.subscribe();
        let user = User::new(username.clone(), client_tx, client_rx);
        //self.users.insert(username, user);
        return user;
    }
}

#[derive(Debug)]
pub(crate) struct User {
    pub(crate) name: String,
    pub(crate) color: Color,
    pub(crate) tx: Sender<String>,
    pub(crate) rx: Receiver<String>,
}

impl User {
    fn new(name: String, tx: Sender<String>, rx: Receiver<String>) -> Self {
        let color_idx = (0..COLORS.len()).choose(&mut thread_rng()).unwrap();
        let color = COLORS[color_idx];
        Self { name, color, tx, rx }
    }

    pub(crate) fn get_prompt(&self) -> String {
        self.color.paint(format!("\r[{}]: ", self.name)).to_string()
    }

    pub(crate) fn format_message(&self, msg: String) -> String {
        format!("{}{}", self.get_prompt(), msg)
    }
}

#[derive(Debug)]
pub(crate) enum CommandType {
    Quit,
    ChangeColor(String),
    Invalid,
}

#[derive(Debug)]
pub(crate) enum Statement {
    Command(CommandType),
    Message(String),
}

const COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Blue,
    Color::Yellow,
    Color::Cyan,
    Color::Purple,
];
