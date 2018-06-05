use slack::RtmClient;

pub trait Bot {
    fn add_handler<T>(&mut self, handler: T) where T: Handler + 'static;
    fn start(&mut self);
}

pub trait Handler {
    fn name(&self) -> String;
    fn can_handle(&self, command: String) -> bool;
    fn handle(&mut self, cli: &RtmClient, message: Message);
}

#[derive(Clone)]
pub struct Message {
    pub channel_id: String,
    pub arguments: Vec<String>
}
