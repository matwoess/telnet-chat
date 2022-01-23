use ansi_term::Colour as Color;
use rand::{thread_rng, prelude::IteratorRandom};
use tokio::sync::broadcast::{Receiver, Sender};

const COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Blue,
    Color::Yellow,
    Color::Cyan,
    Color::Purple,
];

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
    EmptyStatement,
}

#[derive(Debug)]
pub(crate) struct User {
    pub(crate) name: String,
    pub(crate) color: Color,
    pub(crate) tx: Sender<String>,
    pub(crate) rx: Receiver<String>,
}

impl User {
    pub(crate) fn new(name: String, tx: Sender<String>) -> Self {
        let color_idx = (0..COLORS.len()).choose(&mut thread_rng()).unwrap();
        let color = COLORS[color_idx];
        let rx = tx.subscribe();
        Self { name, color, tx, rx }
    }

    pub(crate) fn get_name_prefix(&self) -> String {
        self.color.paint(format!("\r[{}]", self.name)).to_string()
    }

    pub(crate) fn get_prompt(&self) -> String {
        format!("{}: ", self.get_name_prefix())
    }

    pub(crate) fn format_message(&self, msg: String) -> String {
        format!("{}{}", self.get_prompt(), msg)
    }
}
