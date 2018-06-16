use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// A plugin which allows you to add extra functionality to the bot.
pub trait Plugin: Any + Send + Sync {
    /// Get a name describing the `Plugin`.
    fn name(&self) -> &'static str;
    /// Handle inbound message
    fn handle(&mut self, state: &Arc<State>, message: &InboundMessage) -> Result<Vec<OutboundMessage>, String>;
}

pub trait State: Any + Send + Sync {
    fn me(&self) -> Vec<User>;
    fn users(&self) -> Vec<User>;
    fn rooms(&self) -> Vec<Room>;
}

#[derive(Clone, Debug)]
pub struct InboundMessage {
    pub source: Source,
    pub command: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct OutboundMessage {
    pub destination: Source,
    pub content: String,
}

#[derive(Clone, Debug)]
pub enum Source {
    User(User),
    Room(Room),
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub name: String,
    pub attributes: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub attributes: HashMap<String, String>,
}
