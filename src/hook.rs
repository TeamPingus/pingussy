use serenity::all::{Context, Guild, Member, Message};
use serenity::all::standard::{CommandResult, DispatchError};
use serenity::all::standard::macros::hook;
use ansi_term::Color::*;
use crate::CommandCounter;

#[hook]
pub async fn onjoin(_ctx: &Context, _msg: &Message, _member: Member, _guild: &Guild) {
    let _onjoin = "";
}

#[hook]
pub async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    let guild = &msg.guild_id.unwrap().name(ctx).unwrap().to_string();
    println!("Got command '{}' by user '{}' in '{}'", Cyan.paint(command_name), Yellow.paint(&msg.author.name), Green.paint(guild));

    // Increment the number of times this command has been run once. If the command's name does not
    // exist in the counter, add a default value of 0.
    let mut data = ctx.data.write().await;
    let counter = data.get_mut::<CommandCounter>().expect("Expected CommandCounter in TypeMap.");
    let entry = counter.entry(command_name.to_string()).or_insert(0);
    *entry += 1;

    true // if `before` returns false, command processing doesn't happen.
}

#[hook]
pub async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => println!("Processed command '{command_name}'"),
        Err(why) => println!("Command '{command_name}' returned error {why:?}"),
    }
}

#[hook]
pub async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    msg.reply(&ctx.http, format!("Could not find command named '{unknown_command_name}'")).await.expect("TODO: panic message");
}

#[hook]
pub async fn normal_message(_ctx: &Context, msg: &Message) {
    let guild = &msg.guild_id.unwrap().name(_ctx).unwrap().to_string();
    let channel = &msg.channel_id.name(_ctx).await.unwrap().to_string();
    println!("User '{}' in guild '{}' in channel '{}': '{}'", Yellow.paint(&msg.author.name), Green.paint(guild), Blue.paint(channel), Cyan.paint(&msg.content));
}

#[hook]
pub async fn delay_action(ctx: &Context, msg: &Message) {
    // You may want to handle a Discord rate limit if this fails.
    let _ = msg.react(ctx, '⏱').await;
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
    if let DispatchError::Ratelimited(info) = error {
        // We notify them only once.
        if info.is_first_try {
            let _ = msg
                .channel_id
                .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                .await;
        }
    }
}