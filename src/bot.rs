use regex::Regex;
use slack::{Event, EventHandler, Message, RtmClient};
use api;
use api::{Bot, Handler};

pub fn new_slackbot(token: String, trigger: Regex) -> impl Bot {
    SlackBot {
        token,
        trigger,
        handlers: Vec::new(),

        started: false,
    }
}

struct SlackBot {
    token: String,
    trigger: Regex,
    handlers: Vec<Box<Handler>>,

    started: bool,
}

impl SlackBot {
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

impl Bot for SlackBot {
    fn add_handler<T>(&mut self, handler: T) where T: Handler + 'static {
        if self.started {
            panic!("cannot add handler to started bot!");
        }

        debug!("Added handler {}", handler.name());
        self.handlers.push(Box::new(handler));
    }

    fn start(&mut self) {
        info!("Starting..");
        if self.started {
            panic!("cannot start bot, already started!");
        }
        self.started = true;
        match RtmClient::login(&self.token) {
            Err(err) => panic!("Error: {}", err),
            Ok(client) => {

                // Discover slf
                if let Some(slf) = client.start_response().slf.to_owned() {
                    info!("Discovered slf: {}", slf.name.unwrap_or(String::new()));
                }

                // Discover users
                if let Some(users) = client.start_response().users.as_ref() {
                    for v in users.to_owned().into_iter() {
                        info!("Discovered user: {}", v.name.unwrap_or(String::new()));
                    }
                }

                // Discover mpims
                if let Some(mpims) = client.start_response().mpims.as_ref() {
                    for v in mpims.to_owned().into_iter() {
                        info!("Discovered mpim: {}", v.name.unwrap_or(String::new()));
                    }
                }

                // Discover channels
                if let Some(channels) = client.start_response().channels.as_ref() {
                    for v in channels.to_owned().into_iter() {
                        info!("Discovered channel: {}", v.name.unwrap_or(String::new()));
                    }
                }

                // Discover groups
                if let Some(groups) = client.start_response().groups.as_ref() {
                    for v in groups.to_owned().into_iter() {
                        info!("Discovered group: {}", v.name.unwrap_or(String::new()));
                    }
                }

                info!("Started.");
                if let Err(err) = client.run(self) {
                    panic!("Error: {}", err);
                }
            }
        };
    }
}

impl EventHandler for SlackBot {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        match event {
            Event::Message(box_message) => {
                match *box_message {
                    Message::Standard(msg) => {
                        let original = msg.to_owned();

                        let channel_id = msg.channel.unwrap_or(String::new());
                        let text = msg.text.unwrap_or(String::new());

                        let (command, arguments) = self.parse_message(&text);

                        if let Some(c) = command {
                            info!("[{}] command: {}, arguments: {:?}", channel_id, c, arguments);

                            let message = api::Message {
                                channel_id,
                                arguments: arguments.to_owned()
                            };

                            for handler in self.handlers.iter_mut() {
                                if handler.can_handle(c.to_owned()) {
                                    handler.handle(cli, message.to_owned());
                                }
                            }
                        } else {
                            info!("msg: {:#?}", original)
                        }
                    },
                    _ => info!("message: {:#?}", *box_message)
                }
            },
            _ => info!("event: {:#?}", event)
        }
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        info!("Disconnected")
    }

    fn on_connect(&mut self, _cli: &RtmClient) {
        info!("Connected")
    }
}

