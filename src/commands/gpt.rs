use std::{env, time::Duration};

use reqwest::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::{
    builder,
    collector::MessageCollectorBuilder,
    futures::StreamExt,
    model::prelude::{
        command::CommandOptionType,
        interaction::{
            application_command::{
                ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
            },
            InteractionResponseType,
        },
        ChannelId, GuildChannel, Message, UserId,
    },
    prelude::Context,
};

struct GptConversation {
    gpt_chat: GptChat,
    thread: GuildChannel,
    author_id: UserId,
    client: reqwest::Client,
    ctx: Context,
    current_tokens: u32,
}

impl GptConversation {
    async fn new(
        channel_id: ChannelId,
        ctx: Context,
        command: &ApplicationCommandInteraction,
    ) -> GptConversation {
        let channel = ctx.http.get_channel(channel_id.0).await.unwrap();

        if let Err(why) = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("New GPT-3 Chat");
                        message
                    })
            })
            .await
        {
            println!("Cannot respond to slash command: {}", why);
        }

        let msg = command.get_interaction_response(&ctx.http).await.unwrap();

        let thread = channel
            .guild()
            .unwrap()
            .create_public_thread(&ctx.http, msg.id, |f| f.name("GPT-3 Chat"))
            .await
            .unwrap()
            .to_owned();

        let author = command.member.as_ref().unwrap().user.id.clone();
        GptConversation {
            gpt_chat: GptChat::default(),
            client: reqwest::Client::new(),
            thread,
            current_tokens: 8,
            author_id: author,
            ctx,
        }
    }
    async fn listen(&mut self) -> Result<(), reqwest::Error> {
        let mut collector = MessageCollectorBuilder::new(&self.ctx)
            .timeout(Duration::from_secs(6000))
            .author_id(self.author_id)
            .collect_limit(50)
            .channel_id(self.thread.id.0)
            .build();

        while let Some(msg) = collector.next().await {
            let new_message = GptMessage {
                role: Role::user,
                content: msg.content.clone(),
            };

            let returned_message = self.send_message(new_message).await.unwrap();
            let guild = msg.channel(&self.ctx).await.unwrap().guild().unwrap();

            let bytes = returned_message.as_bytes();

            for chunk in bytes.chunks(2000) {
                let chunk = String::from_utf8(chunk.to_vec()).unwrap();
                guild
                    .say(&self.ctx.http, chunk)
                    .await
                    .expect("Failed to send message");
            }
        }
        return Ok(());
    }
    fn update_max_tokens(&mut self, new_message: &GptMessage) {
        let tokens: u32 = (new_message.content.len() as f32 * 1.5).floor() as u32;
        self.gpt_chat.max_tokens = 4096 - (self.current_tokens + tokens);
    }
    async fn send_message(&mut self, new_message: GptMessage) -> Result<String, reqwest::Error> {
        self.update_max_tokens(&new_message);
        println!("Max tokens: {}", self.gpt_chat.max_tokens);
        println!("Current tokens: {}", self.current_tokens);

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

        self.update_current_tokens(&gpt_res);

        let text_response: String =
            serde_json::from_str(&gpt_res["choices"][0]["message"]["content"].to_string()).unwrap();
        let new_message = GptMessage {
            role: Role::assistant,
            content: text_response.clone(),
        };

        self.gpt_chat.messages.push(new_message);

        return Ok(text_response);
    }
    fn update_current_tokens(&mut self, gpt_response: &Value) {
        if gpt_response["error"] != Value::Null {
            println!("Error: {}", gpt_response["error"]["message"]);
            return;
        }

        let tokens: u32 = gpt_response["usage"]["total_tokens"].as_u64().unwrap() as u32;
        self.current_tokens = tokens;
    }
}

#[derive(Serialize, Deserialize, Debug)]
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
            max_tokens: 4000,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
struct GptMessage {
    role: Role,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum Role {
    system,
    assistant,
    user,
}

pub fn register(
    command: &mut builder::CreateApplicationCommand,
) -> &mut builder::CreateApplicationCommand {
    command.name("gpt").description("Start a chat with GPT-3.5")
    // .create_option(|option| {
    //     option
    //         .name("prompt")
    //         .description("a prompt for gpt")
    //         .kind(CommandOptionType::String)
    //         .required(true)
    // })
}
pub async fn run(command: &ApplicationCommandInteraction, ctx: Context) {
    // let _prompt = command
    //     .data
    //     .options
    //     .get(0)
    //     .unwrap()
    //     .resolved
    //     .as_ref()
    //     .unwrap();

    // if let CommandDataOptionValue::String(prompt) = _prompt {
    GptConversation::new(command.channel_id, ctx, command)
        .await
        .listen()
        .await
        .unwrap();
    // }
}
