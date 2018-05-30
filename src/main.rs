extern crate regex;
extern crate slack;
extern crate rand;

use std::env;
use bot::Bot;
use regex::Regex;

mod bot;
mod api;
mod handlers;

const BOT_NAME: &str = "rust_bucket";

fn main() {
    let token = match env::var("SLACK_BOT_TOKEN") {
        Ok(token) => token,
        Err(_) => panic!("Failed to get SLACK_BOT_TOKEN from env"),
    };

    let mut bot = Bot::new(
        BOT_NAME.to_string(),
        token,
        Regex::new(r"^!([^\s!]+?)(\s.+)?$").unwrap()
    );
    bot.add_handler(handlers::CoreyHotline::new());
    bot.start();
}
