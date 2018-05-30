use regex::Regex;
use slack::{Channel, Event, EventHandler, Message, RtmClient};
use api;
use api::Handler;
use std::collections::HashMap;

pub struct Bot {
    name: String,
    token: String,
    trigger: Regex,
    handlers: Vec<Box<Handler>>,

    _started: bool,
    _channels: HashMap<String, Channel>
}

#[allow(dead_code)]
impl Bot {
    pub fn new(name: String, token: String, trigger: Regex) -> Bot {
        Bot {
            name,
            token,
            trigger,
            handlers: Vec::new(),

            _started: false,
            _channels: HashMap::new()
        }
    }

    pub fn add_handler<T>(&mut self, handler: T) where T: Handler + 'static {
        if self._started {
            panic!("cannot add handler to started bot!");
        }

        self.handlers.push(Box::new(handler));
    }

    pub fn start(&mut self) {
        if self._started {
            panic!("cannot start bot, already started!");
        }
        self._started = true;
        match RtmClient::login(&self.token) {
            Err(err) => panic!("Error: {}", err),
            Ok(client) => {
                match client.start_response().channels.as_ref() {
                    Some(channels) => {
                        channels.iter().for_each(|channel| {
                            println!("Loaded channel: {}", channel.to_owned().name.unwrap());
                            self._channels.insert(channel.to_owned().id.unwrap(), channel.to_owned());
                        });
                    },
                    None => panic!("unable to load channels")
                };

                if let Err(err) = client.run(self) {
                    panic!("Error: {}", err);
                }
            }
        };
    }

    fn parse_message(&self, raw: &str) -> (Option<String>, Vec<String>) {
        let mut command: Option<String> = None;
        let mut arguments: Vec<String> = Vec::new();

        if let Some(captures) = self.trigger.captures(raw) {
            command = captures.get(1).map(|c| c.as_str().to_string());
            arguments = captures.get(2).map(|c| c.as_str()).unwrap_or_else(||{ "" })
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
        }

        (command, arguments)
    }
}

impl EventHandler for Bot {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        match event {
            Event::Message(box_message) => {
                match *box_message {
                    Message::Standard(msg) => {
                        let text = msg.text.unwrap_or("".to_owned());
                        let (command, arguments) = self.parse_message(&text);
                        let channel_id = msg.channel.unwrap_or("".to_owned());

                        let channel = self._channels.get(channel_id.as_str());
                        let channel_name = channel
                            .and_then(|c| c.to_owned().name)
                            .unwrap_or_else(|| String::from("<unknown>"));

                        match command {
                            Some(c) => {
                                println!("[{}@{}] command: {}, arguments: {:?}", self.name, channel_name, c, arguments);

                                let message = api::Message {
                                    channel_id,
                                    channel: channel.map(|c| c.clone()),
                                    arguments: arguments.clone()
                                };

                                for handler in self.handlers.iter_mut() {
                                    if handler.can_handle(c.to_owned()) {
                                        handler.handle(cli, message.clone());
                                    }
                                }
                            },
                            _ => {}
                        };
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        println!("RTM API disconnected")
    }

    fn on_connect(&mut self, _cli: &RtmClient) {
        println!("RTM API connected")
    }
}

