mod commands;

use std::env;

use serenity::{
    async_trait,
    framework::StandardFramework,
    model::prelude::{interaction::Interaction, GuildId, Message},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "image" => {
                    command
                        .defer(&ctx.http)
                        .await
                        .expect("Failed to defer command");
                    let res = commands::image::run(&command.data.options).await.unwrap();
                    if let Err(why) = command
                        .edit_original_interaction_response(&ctx.http, |m| m.content(res.0))
                        .await
                    {
                        command.channel_id.say(&ctx.http, why).await.unwrap();
                    }
                }
                "gpt" => commands::gpt::run(&command, ctx).await,
                _ => Err("Unknown command".to_string()).unwrap(),
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: serenity::model::gateway::Ready) {
        let guilds = ctx.cache.guilds();
        for guild in guilds {
            let commands = GuildId::set_application_commands(&guild, &ctx.http, |commands| {
                commands.create_application_command(|command| commands::image::register(command));
                commands.create_application_command(|command| commands::gpt::register(command))
            })
            .await;
        }
        println!("{} is connected!", ready.user.name,);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let framework = StandardFramework::new();
    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .unwrap_or_else(|e| panic!("Err creating client: {:?}", e));

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
    return Ok(());
}
