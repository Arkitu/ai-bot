use tokio;
use dotenv::dotenv;
use std::env;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _:Context, ready:Ready) {
        println!("{} is connected", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        
        msg.channel_id.say(&ctx.http, format!("You said : {}", msg.content)).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").unwrap();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    
    let mut client = Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
