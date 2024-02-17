//! Requires the 'framework' feature flag be enabled in your project's `Cargo.toml`.
//!
//! This can be enabled by specifying the feature in the dependency section:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["framework", "standard_framework"]
//! ```
#![allow(deprecated)]

mod config;

// We recommend migrating to poise, instead of using the standard command framework.
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use serenity::all::{CreateEmbed};

use serenity::async_trait;
use serenity::builder::{CreateMessage, EditChannel};
use serenity::framework::standard::buckets::{LimitedFor};
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{
    help_commands,
    Args,
    BucketBuilder,
    CommandGroup,
    CommandResult,
    Configuration,
    DispatchError,
    HelpOptions,
    StandardFramework,
};
use serenity::gateway::ShardManager;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::id::UserId;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;

// A container type is created for inserting into the Client's `data`, which allows for data to be
// accessible across all events and framework commands, or anywhere else that has a copy of the
// `data` Arc.
struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(about, am_i_admin, ping, some_long_command)]
struct General;

#[group]
#[required_permissions(ADMINISTRATOR)]
// Limit all commands to be guild-restricted.
#[only_in(guilds)]
// Summary only appears when listing multiple groups.
#[summary = "Commands for server admins"]
#[commands(slow_mode, mod_command, commands)]
struct Admins;

// The framework provides two built-in help commands for you to use. But you can also make your own
// customized help command that forwards to the behaviour of either of them.
#[help]
// This replaces the information that a user can pass a command-name as argument to gain specific
// information about it.
#[individual_command_tip = "Hello, this is help!\n\n\
If you want more information about a specific command, just pass the command as argument."]
// Some arguments require a `{}` in order to replace it with contextual information.
// In this case our `{}` refers to a command's name.
#[command_not_found_text = "Could not find: `{}`."]
// Define the maximum Levenshtein-distance between a searched command-name and commands. If the
// distance is lower than or equal the set distance, it will be displayed as a suggestion.
// Setting the distance to 0 will disable suggestions.
#[max_levenshtein_distance(3)]
// When you use sub-groups, Serenity will use the `indention_prefix` to indicate how deeply an item
// is indented. The default value is "-", it will be changed to "+".
#[indention_prefix = "+"]
// On another note, you can set up the help-menu-filter-behaviour.
// Here are all possible settings shown on all possible options.
// First case is if a user lacks permissions for a command, we can hide the command.
#[lacking_permissions = "Hide"]
// If the user is nothing but lacking a certain role, we just display it.
#[lacking_role = "Nothing"]
// The last `enum`-variant is `Strike`, which ~~strikes~~ a command.
#[wrong_channel = "Strike"]
// Serenity will automatically analyse and generate a hint/tip explaining the possible cases of
// ~~strikethrough-commands~~, but only if `strikethrough_commands_tip_in_{dm, guild}` aren't
// specified. If you pass in a value, it will be displayed instead.
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    println!("Got command '{}' by user '{}'", command_name, msg.author.name);

    // Increment the number of times this command has been run once. If the command's name does not
    // exist in the counter, add a default value of 0.
    let mut data = ctx.data.write().await;
    let counter = data.get_mut::<CommandCounter>().expect("Expected CommandCounter in TypeMap.");
    let entry = counter.entry(command_name.to_string()).or_insert(0);
    *entry += 1;

    true // if `before` returns false, command processing doesn't happen.
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => println!("Processed command '{command_name}'"),
        Err(why) => println!("Command '{command_name}' returned error {why:?}"),
    }
}

#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{unknown_command_name}'");
}

#[hook]
async fn normal_message(_ctx: &Context, msg: &Message) {
    println!("Message is not a command '{}'", msg.content);
}

#[hook]
async fn delay_action(ctx: &Context, msg: &Message) {
    // You may want to handle a Discord rate limit if this fails.
    let _ = msg.react(ctx, '⏱').await;
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
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

// You can construct a hook without the use of a macro, too.
// This requires some boilerplate though and the following additional import.
use serenity::futures::future::BoxFuture;
use serenity::FutureExt;
use crate::config::{CONFIG, create_config};

fn _dispatch_error_no_macro<'fut>(
    ctx: &'fut mut Context,
    msg: &'fut Message,
    error: DispatchError,
    _command_name: &str,
) -> BoxFuture<'fut, ()> {
    async move {
        if let DispatchError::Ratelimited(info) = error {
            if info.is_first_try {
                let _ = msg
                    .channel_id
                    .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                    .await;
            }
        };
    }
    .boxed()
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    if Path::new("stuff.conf").exists() {
        println!("Config already exists");
    } else {
        create_config().expect("failed to create config");
    }
    let dc_token = &CONFIG.token;
    let token = env::var("DISCORD_TOKEN").unwrap_or(dc_token.parse().expect("No DISCORD_TOKEN? *insert megamind*")).to_string();

    let http = Http::new(&token);

    // We will fetch your bot's owners and id
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else if let Some(owner) = &info.owner {
                owners.insert(owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        // Set a function to be called prior to each command execution. This provides the context
        // of the command, the message that was received, and the full name of the command that
        // will be called.
        //
        // Avoid using this to determine whether a specific command should be executed. Instead,
        // prefer using the `#[check]` macro which gives you this functionality.
        //
        // **Note**: Async closures are unstable, you may use them in your application if you are
        // fine using nightly Rust. If not, we need to provide the function identifiers to the
        // hook-functions (before, after, normal, ...).
        .before(before)
        // Similar to `before`, except will be called directly _after_ command execution.
        .after(after)
        // Set a function that's called whenever an attempted command-call's command could not be
        // found.
        .unrecognised_command(unknown_command)
        // Set a function that's called whenever a message is not a command.
        .normal_message(normal_message)
        // Set a function that's called whenever a command's execution didn't complete for one
        // reason or another. For example, when a user has exceeded a rate-limit or a command can
        // only be performed by the bot owner.
        .on_dispatch_error(dispatch_error)
        // Can't be used more than once per 5 seconds:
        .bucket("complicated",
            BucketBuilder::default().limit(2).time_span(30).delay(5)
                // The target each bucket will apply to.
                .limit_for(LimitedFor::Channel)
                // The maximum amount of command invocations that can be delayed per target.
                // Setting this to 0 (default) will never await/delay commands and cancel the invocation.
                .await_ratelimits(1)
                // A function to call when a rate limit leads to a delay.
                .delay_action(delay_action)
        ).await
        // The `#[group]` macro generates `static` instances of the options set for the group.
        // They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
        // #name is turned all uppercase
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&ADMINS_GROUP);

    framework.configure(
        Configuration::new().with_whitespace(true)
            .on_mention(Some(bot_id))
            .prefix("c!")
            // In this case, if "," would be first, a message would never be delimited at ", ",
            // forcing you to trim your arguments if you want to avoid whitespaces at the start of
            // each.
            .delimiters(vec![", ", ","])
            // Sets the bot's owners. These will be used for commands that are owners only.
            .owners(owners),
    );

    // For this example to run properly, the "Presence Intent" and "Server Members Intent" options
    // need to be enabled.
    // These are needed so the `required_permissions` macro works on the commands that need to use
    // it.
    // You will need to enable these 2 options on the bot application, and possibly wait up to 5
    // minutes.
    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

// Commands can be created via the attribute `#[command]` macro.
#[command]
// Options are passed via subsequent attributes.
// Make this command use the "complicated" bucket.
#[bucket = "complicated"]
async fn commands(ctx: &Context, msg: &Message) -> CommandResult {
    let mut contents = "Commands used:\n".to_string();

    let data = ctx.data.read().await;
    let counter = data.get::<CommandCounter>().expect("Expected CommandCounter in TypeMap.");

    for (name, amount) in counter {
        writeln!(contents, "- {name}: {amount}")?;
    }

    msg.channel_id.say(&ctx.http, &contents).await?;

    Ok(())
}

/*
// Repeats what the user passed as argument but ensures that user and role mentions are replaced
// with a safe textual alternative.
// In this example channel mentions are excluded via the `ContentSafeOptions`.
#[command]
async fn say(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    match args.single_quoted::<String>() {
        Ok(x) => {
            let settings = if let Some(guild_id) = msg.guild_id {
                // By default roles, users, and channel mentions are cleaned.
                ContentSafeOptions::default()
                    // We do not want to clean channal mentions as they do not ping users.
                    .clean_channel(false)
                    // If it's a guild channel, we want mentioned users to be displayed as their
                    // display name.
                    .display_as_member_from(guild_id)
            } else {
                ContentSafeOptions::default().clean_channel(false).clean_role(false)
            };

            let content = content_safe(&ctx.cache, x, &settings, &msg.mentions);

            msg.channel_id.say(&ctx.http, &content).await?;

            return Ok(());
        },
        Err(_) => {
            msg.reply(ctx, "An argument is required to run this command.").await?;
            return Ok(());
        },
    };
}
*/

#[command]
async fn some_long_command(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    msg.channel_id.say(&ctx.http, &format!("Arguments: {:?}", args.rest())).await?;

    Ok(())
}

#[command]
// Limits the usage of this command to roles named:
#[allowed_roles("mods", "ultimate neko")]
async fn about_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
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
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "This is the pingussy bot :-)").await?;

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
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

    dbg!(&runner.latency);
    msg.reply(ctx, &format!("{:?}ms", &runner.latency.unwrap().as_millis())).await?;

    Ok(())
}
// We could also use #[required_permissions(ADMINISTRATOR)] but that would not let us reply when it
// fails.
#[command]
async fn am_i_admin(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
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
async fn slow_mode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
#[sub_commands(sub, kick)]
async fn mod_command(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let sub = "`sub`: This is a sub command";
    let kick = "`kick`: Kicks a user (WIP)";
    let embed = CreateEmbed::new()
        .title("Available commands:")
        .description(format!("{}\n{}", sub, kick));
        //.field("Available commands: ", false);
    let builder = CreateMessage::new().content("test").tts(false).embed(embed);

    msg.channel_id.send_message(&ctx.http, builder).await.expect("TODO: panic message");

    Ok(())
}


#[command]
#[aliases("sub-command", "secret")]
#[description("This is `mod`'s sub-command.")]
async fn sub(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "This is a sub function!").await?;

    Ok(())
}
#[command]
#[description("Kicks a member")]
async fn kick(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "this will kick a member, once i figure out, how to do that").await?;

    Ok(())
}