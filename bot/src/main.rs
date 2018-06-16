extern crate api;
extern crate env_logger;
extern crate libloading as lib;
#[macro_use]
extern crate log;
extern crate regex;
extern crate slack;

use bot::Bot;
use regex::Regex;
use std::env;

mod bot;
mod slack_bot;

fn main() {
    env_logger::init();

    info!("Initializing..");

    let token = match env::var("SLACK_BOT_TOKEN") {
        Ok(token) => token,
        Err(_) => panic!("Failed to get SLACK_BOT_TOKEN from env"),
    };
    debug!("Found token: {}", token);

    let base_path = env::current_exe().unwrap().parent().unwrap().to_path_buf();

    let mut b = slack_bot::create(token, Regex::new(r"^!([^\s!]+?)(\s.+)?$").unwrap());
    unsafe {
        let mut plugin_path = base_path.clone();
        plugin_path.push("libcorey_hotline.dylib");
        if let Err(e) = b.plugin(plugin_path.to_str().unwrap()) {
            panic!("Failed to load plugin: {:?}", e);
        }
    }
    b.start();
}
