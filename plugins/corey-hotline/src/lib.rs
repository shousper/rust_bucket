extern crate api;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;

use api::{HandleError, HandleResult, InboundMessage, OutboundMessage, Plugin, State};
use rand::{thread_rng, Rng};

mod data;

#[derive(Clone, Debug)]
struct CoreyHotlinePlugin {
    count: i32,
}

#[no_mangle]
pub extern "C" fn new_plugin() -> *const (Plugin + 'static) {
    env_logger::init();
    debug!("Loaded!");
    Box::into_raw(Box::new(CoreyHotlinePlugin { count: 0 }))
}

impl CoreyHotlinePlugin {
    fn random_response() -> &'static str { thread_rng().choose(&data::RESPONSES).unwrap() }
    fn command_corey(&mut self, message: &InboundMessage) -> Vec<OutboundMessage> {
        self.count += 1;
        info!("Executing corey for the {} time..", self.count);
        vec![
            api::OutboundMessage {
                destination: message.source.clone(),
                content: CoreyHotlinePlugin::random_response().to_string(),
            },
        ]
    }
}

impl Plugin for CoreyHotlinePlugin {
    fn name(&self) -> &'static str {
        "CoreyHotline"
    }
    fn handle(&mut self, _state: &State, message: &InboundMessage) -> HandleResult {
        info!("Received {} command", message.command);
        match message.command.as_str() {
            "corey" => Ok(self.command_corey(message)),
            _ => Err(HandleError::Unhandled),
        }
    }
}
