use api::{InboundMessage, Plugin, Source, Room, State, User};
use bot::{Bot, PluginError};
use lib::{Library};
use regex::Regex;
use slack::{Event, EventHandler, Message, RtmClient};
use std::collections::HashMap;
use api::OutboundMessage;
use api::HandleError;
use api::HandleResult;
use std::env;

type PluginCreate = unsafe fn() -> *mut Plugin;

pub fn create(token: String, trigger: Regex) -> impl Bot {
    SlackBot {
        token,
        trigger,

        libs: HashMap::new(),
        plugins: Vec::new(),

        state: SlackState::new(),
        started: false,
    }
}

#[derive(Clone, Debug)]
struct SlackState {
    me: Option<User>,
    users: Vec<User>,
    rooms: Vec<Room>,
}

impl SlackState {
    fn new() -> Self {
        SlackState {
            me: None,
            users: Vec::new(),
            rooms: Vec::new(),
        }
    }
}

impl State for SlackState {
    fn me(&self) -> Option<User> {
        self.me.clone()
    }

    fn users(&self) -> Vec<User> {
        self.users.clone()
    }

    fn rooms(&self) -> Vec<Room> {
        self.rooms.clone()
    }
}

struct SlackBot {
    token: String,
    trigger: Regex,

    libs: HashMap<String, Library>,
    plugins: Vec<Box<Plugin>>,

    state: SlackState,
    started: bool,
}

impl SlackBot {
    unsafe fn load_plugin(&mut self, lib: &Library) -> Result<(), PluginError> {
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

        match self.core_commands(&message.clone()) {
            Ok(outbound_messages) => {
                SlackBot::send_responses(cli, outbound_messages);
                return;
            },
            Err(err) => {
                match err {
                    HandleError::Unhandled => {},
                    _ => {
                        error!("Failed to handle message: {:?}", err);
                        SlackBot::send_responses(cli, vec![
                            OutboundMessage {
                                destination: message.source.clone(),
                                content: format!("Whoops! {:?}", err),
                            }
                        ])
                    }
                }
            }
        }

        for mut plugin in self.plugins.iter_mut() {
            info!("Trying {:?} first!", plugin.name());
            match plugin.handle(&self.state.clone(), &message.clone()) {
                Ok(outbound_messages) => {
                    info!("Plugin {} responded with {} messages", plugin.name(), outbound_messages.len());
                    SlackBot::send_responses(cli, outbound_messages);
                },
                Err(err) => {
                    match err {
                        HandleError::Unhandled => {},
                        _ => {
                            error!("Failed to handle message: {:?}", err);
                            SlackBot::send_responses(cli, vec![
                                OutboundMessage {
                                    destination: message.source.clone(),
                                    content: format!("Whoops! {:?}", err),
                                }
                            ])
                        }
                    }
                }
            }
        }
    }

    fn core_commands(&mut self, message: &InboundMessage) -> HandleResult {
        match message.command.as_str() {
            "version" => Ok(vec![
                OutboundMessage {
                    destination: message.source.clone(),
                    content: "I am version 1.0".to_string(),
                }
            ]),
            "load" => unsafe {
                match self.plugin(message.arguments[0].as_str()) {
                    Ok(_) => Ok(vec![
                        OutboundMessage {
                            destination: message.source.clone(),
                            content: format!("Loaded {}", message.arguments[0]),
                        }
                    ]),
                    Err(e) => Err(HandleError::Unexpected(format!("{:?}", e))),
                }
            }
            _ => Err(HandleError::Unhandled),
        }
    }

    fn send_responses(cli: &RtmClient, msgs: Vec<OutboundMessage>) {
        for msg in msgs {
            match msg.destination {
                Source::Room(room) => {
                    if let Err(e) = cli.sender().send_message(room.id.as_str(), msg.content.as_str()) {
                        error!("Unable to send message {} to {}: {:?}", msg.content, room.id, e);
                    }
                },
                _ => {}
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
    unsafe fn plugin(&mut self, name: &str) -> Result<(), PluginError> {
        debug!("Loading {} plugin", name);
        if self.libs.contains_key(&name.to_string()) {
            debug!("Plugin {} already loaded", name);
            return Err(PluginError::AlreadyLoaded);
        }

        let base_path = env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let mut path_buf = base_path.clone();
        path_buf.push(format!("lib{}.dylib", name));
        let plugin_path = path_buf.to_str().unwrap();

        debug!("Loading {} plugin from {}", name, plugin_path);
        match Library::new(plugin_path) {
            Ok(lib) => {
                self.load_plugin(&lib)?;
                self.libs.insert(name.to_string(), lib);
                info!("Loaded {} plugin", name);
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

        match RtmClient::login(&self.token) {
            Err(err) => panic!("Error: {}", err),
            Ok(client) => {
                // Discover slf
                if let Some(slf) = client.start_response().slf.to_owned() {
                    self.state.me = Some(User {
                        id: slf.id.unwrap_or_else(String::new),
                        username: slf.name.unwrap_or_else(String::new),
                        name: slf.real_name.unwrap_or_else(String::new),
                        attributes: HashMap::new(),
                    });
                }

                // Discover users
                if let Some(users) = client.start_response().users.as_ref() {
                    for u in users.to_owned().into_iter() {
                        self.state.users.push(User {
                            id: u.id.unwrap_or_else(String::new),
                            username: u.name.unwrap_or_else(String::new),
                            name: u.real_name.unwrap_or_else(String::new),
                            attributes: HashMap::new(),
                        });
                    }
                }

                // Discover mpims
                if let Some(mpims) = client.start_response().mpims.as_ref() {
                    for r in mpims.to_owned().into_iter() {
                        self.state.rooms.push(Room {
                            id: r.id.unwrap_or_else(String::new),
                            name: r.name.unwrap_or_else(String::new),
                            attributes: [(String::from("type"), String::from("mpim"))].iter().cloned().collect(),
                        });
                    }
                }

                // Discover channels
                if let Some(channels) = client.start_response().channels.as_ref() {
                    for r in channels.to_owned().into_iter() {
                        self.state.rooms.push(Room {
                            id: r.id.unwrap_or_else(String::new),
                            name: r.name.unwrap_or_else(String::new),
                            attributes: [(String::from("type"), String::from("channel"))].iter().cloned().collect(),
                        });
                    }
                }

                // Discover groups
                if let Some(groups) = client.start_response().groups.as_ref() {
                    for r in groups.to_owned().into_iter() {
                        self.state.rooms.push(Room {
                            id: r.id.unwrap_or_else(String::new),
                            name: r.name.unwrap_or_else(String::new),
                            attributes: [(String::from("type"), String::from("group"))].iter().cloned().collect(),
                        });
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
