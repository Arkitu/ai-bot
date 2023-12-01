use llm_chain::options::ModelRef;
use llm_chain::{executor, prompt, parameters, options};
use llm_chain_llama::Executor;
use tokio;
use dotenv::dotenv;
use std::env;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

mod llama_cpp_executor;

struct Handler {
    llm_exec: Executor
}

impl Handler {
    pub fn new() -> Self {
        let opts = options!(
            Model: ModelRef::from_path("./ggml-alpaca-7b-q4.bin"), // Notice that we reference the model binary path
            ModelType: "llama",
            MaxContextSize: 512_usize,
            NThreads: 4_usize,
            MaxTokens: 0_usize,
            TopK: 40_i32,
            TopP: 0.95,
            TfsZ: 1.0,
            TypicalP: 1.0,
            Temperature: 0.8,
            RepeatPenalty: 1.1,
            RepeatPenaltyLastN: 64_usize,
            FrequencyPenalty: 0.0,
            PresencePenalty: 0.0,
            Mirostat: 0_i32,
            MirostatTau: 5.0,
            MirostatEta: 0.1,
            PenalizeNl: true,
            StopSequence: vec!["\n".to_string()]
        );
        let exec = executor!(llama, opts).unwrap();
        Self {
            llm_exec: exec
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _:Context, ready:Ready) {
        println!("{} is connected", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache) {
            return
        }
        println!("Responding to {}…", msg.author.name);
        let res = prompt!(
            "System: You are a discord bot assistant, interacting with discord users",
            &format!("{}: {}", msg.author.name, msg.content)
        ).run(&parameters!(), &self.llm_exec).await.unwrap();
        msg.channel_id.say(&ctx.http, res.to_immediate().await.unwrap().to_string()).await.unwrap();
        println!("Response done for {} !", msg.author.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let discord_token = env::var("DISCORD_TOKEN").unwrap();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    
    let mut client = Client::builder(&discord_token, intents).event_handler(Handler::new()).await.expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
