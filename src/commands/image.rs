use std::env;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::builder;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    CommandDataOption, CommandDataOptionValue,
};

#[derive(Serialize, Deserialize)]
struct DalleBody {
    prompt: String,
    n: u8,
    size: String,
}
impl DalleBody {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            n: 1,
            size: "1024x1024".to_string(),
        }
    }
}

pub fn register(
    command: &mut builder::CreateApplicationCommand,
) -> &mut builder::CreateApplicationCommand {
    command
        .name("image")
        .description("Generate an image with DALL-E")
        .create_option(|option| {
            option
                .name("prompt")
                .description("A prompt for the image")
                .kind(CommandOptionType::String)
                .required(true)
        })
}
pub async fn run(_options: &[CommandDataOption]) -> Option<(String, String)> {
    let _prompt = _options.get(0).unwrap().resolved.as_ref().unwrap();

    if let CommandDataOptionValue::String(prompt) = _prompt {
        let body = DalleBody::new(prompt.to_string());
        let body = serde_json::to_string(&body).unwrap();

        let res: Value = match reqwest::Client::new()
            .post("https://api.openai.com/v1/images/generations")
            .header(
                "Authorization",
                format!("Bearer {}", env::var("OPENAI_KEY").expect("token")),
            )
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap()
            .json()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return Some((
                    format!("An error occurred. Please try again. {}", e),
                    prompt.to_owned(),
                ));
            }
        };

        if res["error"] != Value::Null {
            if res["error"]["message"].to_string().contains("is too long") {
                return Some((
                    "The prompt is too long. Please shorten it.".to_string(),
                    prompt.to_owned(),
                ));
            } else {
                return Some((
                    "An error occurred. Please try again.".to_string(),
                    prompt.to_owned(),
                ));
            }
        }

        let image = res["data"][0]["url"].as_str().unwrap().to_string();
        Some((image, prompt.to_owned()))
    } else {
        None
    }
}
