use api::{InboundMessage, Plugin, Source, Room, State, User};
use bot::{Bot, PluginError};
use lib::{Library};
use regex::Regex;
use slack::{Event, EventHandler, Message, RtmClient};
use std::collections::HashMap;
use std::sync::Arc;

type PluginCreate = unsafe fn() -> *mut Plugin;

pub fn create(token: String, trigger: Regex) -> impl Bot {
    SlackBot {
        token,
        trigger,

        libs: Vec::new(),
        plugins: Vec::new(),

        state: Arc::new(SlackState {}),
        started: false,
    }
}

struct SlackState;

impl State for SlackState {
    fn me(&self) -> Vec<User> {
        unimplemented!()
    }

    fn users(&self) -> Vec<User> {
        unimplemented!()
    }

    fn rooms(&self) -> Vec<Room> {
        unimplemented!()
    }
}

struct SlackBot {
    token: String,
    trigger: Regex,

    libs: Vec<Library>,
    plugins: Vec<Box<Plugin>>,

    state: Arc<State>,
    started: bool,
}

impl SlackBot {
    unsafe fn load_plugin(&mut self, lib: &Library) -> Result<(), PluginError> {
        info!("Loading plugin from {:?}", lib);
        match lib.get::<PluginCreate>(b"new_plugin") {
            Ok(constructor) => {
                // Construct plugin
                let raw = constructor();
                let plugin: Box<Plugin> = Box::from_raw(raw);
                // Store and refetch
                self.plugins.push(plugin);
                Ok(())
            },
            Err(_) => Err(PluginError::InvalidPluginConstructor)
        }
    }

    fn on_standard_message(&mut self, cli: &RtmClient, channel_id: String, user: String, text: String) {
        let (opt_command, arguments) = self.parse_message(&text);
        if let None = opt_command {
            info!("[UNHANDLED] {} @ {} : {}", channel_id, user, text);
            return;
        }

        let command = opt_command.unwrap();
        info!("[{}] command: {}, arguments: {:?}", channel_id, command, arguments);

        let message = InboundMessage {
            source: Source::Room(Room {
                id: channel_id,
                name: String::new(),
                attributes: HashMap::new(),
            }),
            command,
            arguments,
        };

        for mut plugin in self.plugins.iter_mut() {
            info!("Trying {:?} first!", plugin.name());
            match plugin.handle(&self.state.clone(), &message.clone()) {
                Ok(outbound_messages) => {
                    info!("Plugin {} responded with {} messages", plugin.name(), outbound_messages.len());
                    for msg in outbound_messages {
                        match msg.destination {
                            Source::Room(room) => {
                                if let Err(e) = cli.sender().send_message(room.id.as_str(), msg.content.as_str()) {
                                    error!("Unable to send message {} to {}: {:?}", msg.content, room.id, e);
                                }
                            },
                            _ => {}
                        }
                    }
                },
                Err(error) => error!("Handling message: {:?}", error)
            }
        }
    }

    fn parse_message<'t>(&'t self, raw: &'t str) -> (Option<String>, Vec<String>) {
        let mut command: Option<String> = None;
        let mut arguments: Vec<String> = Vec::new();

        if let Some(captures) = self.trigger.captures(raw) {
            command = captures.get(1).map(|c| c.as_str().to_string());
            arguments = captures
                .get(2)
                .map(|c| c.as_str())
                .unwrap_or_else(|| "")
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
        }

        (command, arguments)
    }
}

impl Bot for SlackBot {
    unsafe fn plugin(&mut self, filename: &str) -> Result<(), PluginError> {
        info!("Loading library");
        match Library::new(filename) {
            Ok(lib) => {
                self.load_plugin(&lib)?;
                self.libs.push(lib);
                Ok(())
            },
            Err(e) => {
                error!("Unable to load library: {:#?}", e);
                Err(PluginError::InvalidLibrary)
            }
        }
    }

    fn start(&mut self) {
        info!("Starting..");
        if self.started {
            panic!("cannot start bot, already started!");
        }
        self.started = true;

        info!("With {} plugins..", self.plugins.len());
        for p in self.plugins.iter() {
            info!("> Plugin: {}", p.name());
        }

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
            Event::Message(box_message) => match *box_message {
                Message::Standard(msg) => self.on_standard_message(
                    cli,
                    msg.channel.unwrap_or_else(|| { String::new() }),
                    msg.user.unwrap_or_else(|| { String::new() }),
                    msg.text.unwrap_or_else(|| { String::new() })
                ),
                _ => info!("message: {:#?}", *box_message),
            },
            _ => info!("event: {:#?}", event),
        }
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        info!("Disconnected")
    }

    fn on_connect(&mut self, _cli: &RtmClient) {
        info!("Connected")
    }
}
