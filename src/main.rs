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
mod hook;
mod commands;

// We recommend migrating to poise, instead of using the standard command framework.
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use std::sync::Arc;
use serenity::all::CreateMessage;

use crate::commands::*;
use serenity::async_trait;
use serenity::framework::standard::buckets::{LimitedFor};
use serenity::framework::standard::macros::{group, help};
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
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "~hidden dm_me" {
            let msg_content = "You have found a hidden function. It doesn't do anything yet, just exists for testing";
            let builder = CreateMessage::new().content(msg_content);

            if let Err(why) = msg.author.direct_message(&ctx, builder).await {
                println!("Err sending help: {why:?}");
                let _ = msg.reply(&ctx, "There was an error DMing you help.").await;
            };
        }
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(about, am_i_admin, ping, some_long_command, reaction)]
struct General;

#[group]
#[required_permissions(ADMINISTRATOR)]
// Limit all commands to be guild-restricted.
#[only_in(guilds)]
// Summary only appears when listing multiple groups.
#[summary = "Commands for server admins"]
#[commands(slow_mode, mod_command, about_role)]
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
        .before(hook::before)
        // Similar to `before`, except will be called directly _after_ command execution.
        .after(hook::after)
        // Set a function that's called whenever an attempted command-call's command could not be
        // found.
        .unrecognised_command(hook::unknown_command)
        // Set a function that's called whenever a message is not a command.
        .normal_message(hook::normal_message)
        // Set a function that's called whenever a command's execution didn't complete for one
        // reason or another. For example, when a user has exceeded a rate-limit or a command can
        // only be performed by the bot owner.
        .on_dispatch_error(hook::dispatch_error)
        // Can't be used more than once per 5 seconds:
        .bucket("complicated",
            BucketBuilder::default().limit(2).time_span(30).delay(5)
                // The target each bucket will apply to.
                .limit_for(LimitedFor::Channel)
                // The maximum amount of command invocations that can be delayed per target.
                // Setting this to 0 (default) will never await/delay commands and cancel the invocation.
                .await_ratelimits(1)
                // A function to call when a rate limit leads to a delay.
                .delay_action(hook::delay_action)
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