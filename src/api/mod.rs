use slack::{Channel, RtmClient};

pub trait Handler {
    fn can_handle(&self, command: String) -> bool;
    fn handle(&mut self, cli: &RtmClient, message: Message);
}

#[derive(Clone)]
pub struct Message {
    pub channel_id: String,
    pub channel: Option<Channel>,
    pub arguments: Vec<String>
}
