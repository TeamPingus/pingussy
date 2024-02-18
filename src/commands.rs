use serenity::all::{Context, CreateEmbed, CreateMessage, EditChannel, Message, Permissions};
use serenity::all::standard::{Args, CommandResult};
use serenity::all::standard::macros::command;
use crate::{ShardManagerContainer};

#[command]
pub async fn some_long_command(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id.say(&ctx.http, &format!("Arguments: {:?}", args.rest())).await?;

    Ok(())
}

#[command]
// Limits the usage of this command to roles named:
pub async fn about_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let role_name = args.rest();
    let to_send = match msg.guild(&ctx.cache).as_deref().and_then(|g| g.role_by_name(role_name)) {
        Some(role_id) => format!("Role-ID: {role_id}"),
        None => format!("Could not find role name: {role_name:?}"),
    };

    if let Err(why) = msg.channel_id.say(&ctx.http, to_send).await {
        println!("Error sending message: {why:?}");
    }

    Ok(())
}

#[command]
pub async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "This is the pingussy bot :-)").await?;

    Ok(())
}

#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager").await?;

            return Ok(());
        },
    };

    let runners = shard_manager.runners.lock().await;
    let runner = match runners.get(&ctx.shard_id) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        },
    };

    // dbg!(&runner.latency);
    msg.reply(ctx, &format!("{:?}ms", &runner.latency.unwrap().as_millis())).await?;

    Ok(())
}
// We could also use #[required_permissions(ADMINISTRATOR)] but that would not let us reply when it
// fails.
#[command]
pub async fn am_i_admin(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let is_admin = if let (Some(member), Some(guild)) = (&msg.member, msg.guild(&ctx.cache)) {
        member.roles.iter().any(|role| {
            guild.roles.get(role).is_some_and(|r| r.has_permission(Permissions::ADMINISTRATOR))
        })
    } else {
        false
    };

    if is_admin {
        msg.channel_id.say(&ctx.http, "Yes, you are.").await?;
    } else {
        msg.channel_id.say(&ctx.http, "No, you are not. get rekt").await?;
    }

    Ok(())
}

#[command]
pub async fn slow_mode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let say_content = if let Ok(slow_mode_rate_seconds) = args.single::<u16>() {
        let builder = EditChannel::new().rate_limit_per_user(slow_mode_rate_seconds);
        if let Err(why) = msg.channel_id.edit(&ctx.http, builder).await {
            println!("Error setting channel's slow mode rate: {why:?}");

            format!("Failed to set slow mode to `{slow_mode_rate_seconds}` seconds.")
        } else {
            format!("Successfully set slow mode rate to `{slow_mode_rate_seconds}` seconds.")
        }
    } else if let Some(channel) = msg.channel_id.to_channel_cached(&ctx.cache) {
        let slow_mode_rate = channel.rate_limit_per_user.unwrap_or(0);
        format!("Current slow mode rate is `{slow_mode_rate}` seconds.")
    } else {
        "Failed to find channel in cache.".to_string()
    };

    msg.channel_id.say(&ctx.http, say_content).await?;

    Ok(())
}

#[command("mod")]
#[sub_commands(sub, kick, info)]
pub async fn mod_command(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let sub = "`sub`: This is a sub command";
    let kick = "`kick`: Kicks a user (WIP)";
    let info = "`info`: Give user info";
    let embed = CreateEmbed::new()
        .title("Available commands:")
        .description(format!("{}\n{}\n{}", sub, kick, info));
    //.field("Available commands: ", false);
    let builder = CreateMessage::new().content("test").tts(false).embed(embed);

    msg.channel_id.send_message(&ctx.http, builder).await.expect("TODO: panic message");

    Ok(())
}


#[command]
#[aliases("sub-command", "secret")]
#[description("This is `mod`'s sub-command.")]
pub async fn sub(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "This is a sub function!").await?;

    Ok(())
}
#[command]
#[description("Kicks a member")]
pub async fn kick(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "this will kick a member, once i figure out, how to do that").await?;

    Ok(())
}

#[command]
#[description("idk")]
pub async fn info(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let member_name = args.rest();
    let to_send = match msg.guild(&ctx.cache).as_deref().and_then(|g| g.member_named(member_name)) {
        Some(member_id) => format!("Member-ID: {member_id}"),
        None => format!("Could not find member name: {member_name:?}"),
    };
    if let Err(why) = msg.channel_id.say(&ctx.http, to_send).await {
        println!("Error sending message: {why:?}");
    }
    Ok(())
}

#[command("react")]
pub async fn reaction(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.react(ctx, '🚀').await?;

    Ok(())
}