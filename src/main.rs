extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rand;
extern crate regex;
extern crate slack;

use api::Bot;
use std::env;
use regex::Regex;

mod api;
mod bot;
mod handlers;

fn main() {
    env_logger::init();

    info!("Initializing..");

    let token = match env::var("SLACK_BOT_TOKEN") {
        Ok(token) => token,
        Err(_) => panic!("Failed to get SLACK_BOT_TOKEN from env"),
    };
    debug!("Found token: {}", token);

    let mut b = bot::new_slackbot(
        token,
        Regex::new(r"^!([^\s!]+?)(\s.+)?$").unwrap()
    );
    b.add_handler(handlers::new_corey_hotline());
    b.start();
}
