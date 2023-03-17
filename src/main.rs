use std::{env, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::{
    async_trait,
    collector::MessageCollectorBuilder,
    framework::StandardFramework,
    futures::StreamExt,
    model::prelude::{GuildChannel, Message},
    prelude::{Context, EventHandler, GatewayIntents},
    Client,
};

struct GptConversation {
    gpt_chat: GptChat,
    thread: GuildChannel,
    author_id: u64,
    client: reqwest::Client,
    ctx: Context,
}

impl GptConversation {
    async fn new(msg: Message, ctx: Context) -> GptConversation {
        let channel = msg.channel(&ctx).await.unwrap();

        let thread = channel
            .guild()
            .unwrap()
            .create_public_thread(&ctx.http, msg.id, |f| f.name("GPT-3 Chat"))
            .await
            .unwrap();

        GptConversation {
            gpt_chat: GptChat::default(),
            client: reqwest::Client::new(),
            thread,
            author_id: msg.author.id.0,
            ctx,
        }
    }
    async fn listen(&mut self) -> Result<(), reqwest::Error> {
        let mut collector = MessageCollectorBuilder::new(&self.ctx)
            .timeout(Duration::from_secs(600))
            .author_id(self.author_id)
            .collect_limit(5u32)
            .channel_id(self.thread.id.0)
            .build();

        while let Some(msg) = collector.next().await {
            let new_message = GptMessage {
                role: Role::user,
                content: msg.content.clone(),
            };
            let returned_message = self.send_message(new_message).await.unwrap();

            msg.channel(&self.ctx)
                .await
                .unwrap()
                .guild()
                .unwrap()
                .say(&self.ctx, returned_message)
                .await
                .unwrap();
        }
        return Ok(());
    }
    async fn send_message(&mut self, new_message: GptMessage) -> Result<String, reqwest::Error> {
        self.gpt_chat.messages.push(new_message);
        let chat = serde_json::to_string(&self.gpt_chat).unwrap();
        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header(
                "Authorization",
                format!("Bearer {}", env::var("OPENAI_KEY").expect("token")),
            )
            .header("Content-Type", "application/json")
            .body(chat)
            .send()
            .await;

        let gpt_res: Value = res.unwrap().json().await?;

        let text_response: String =
            serde_json::from_str(&gpt_res["choices"][0]["message"]["content"].to_string()).unwrap();
        let new_message = GptMessage {
            role: Role::assistant,
            content: text_response.clone(),
        };

        self.gpt_chat.messages.push(new_message);

        return Ok(text_response);
    }
}

#[derive(Serialize, Deserialize)]
struct GptChat {
    model: String,
    messages: Vec<GptMessage>,
    max_tokens: u32,
}

impl GptChat {
    fn default() -> GptChat {
        GptChat {
            model: "gpt-3.5-turbo".to_string(),
            messages: Vec::new(),
            max_tokens: 2096,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct GptMessage {
    role: Role,
    content: String,
}

#[derive(Serialize, Deserialize)]
enum Role {
    system,
    assistant,
    user,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        if msg.content.contains("new chat") {
            GptConversation::new(msg, ctx).await.listen().await.unwrap();
        }
    }
    async fn ready(&self, _: Context, ready: serenity::model::gateway::Ready) {
        println!("{} is connected!", ready.user.name);
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
