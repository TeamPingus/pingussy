#[macro_use]
extern crate lazy_static;

use std::path::Path;

use serenity::async_trait;
use serenity::client::EventHandler;
pub use serenity::prelude::GatewayIntents;
pub use serenity::{client::Context,
                   framework::{
        standard::{
            macros::{command, group},
            CommandResult,
        },
        StandardFramework,
    },
    model::channel::Message,
    Client,
};
use serenity::model::channel::Embed;

use crate::config::{create_config, CONFIG};

mod config;

#[group]
#[commands(ping)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    if Path::new("stuff.conf").exists() {
        println!("Config already exists");
    } else {
        create_config().expect("failed to create config");
    }

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("c!")) // prefix to "c!"
        .group(&GENERAL_GROUP);

    let token = &CONFIG.token;
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    println!("Created client");

    if let Err(why) = client.start().await {
        println!("An error occurred running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    println!("sent Pong!");
    Ok(())
}



/*
pub fn fake() {
    let embed_funny = Embed::fake(|e| {
        e.title("Haha funny").description("69420").field(
            "lOl",
            "ehhh, idk haha funny",
            false,
        )
    });
}
*/
