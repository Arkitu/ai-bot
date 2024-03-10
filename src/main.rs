use llm_chain::options::{Options, Opt};
use llm_chain::prompt::Prompt;
use llm_chain::traits::Executor as ExecutorTrait;
use serenity::all::CreateMessage;
use tokio;
use dotenv::dotenv;
use std::{env, vec};
use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

mod llama_cpp_executor;
use llama_cpp_executor::Executor;


struct Handler {
    exec: Executor,
    busy: Arc<Mutex<()>>
}

impl Handler {
    pub fn new() -> Self {
        let exec = Executor::new().unwrap();
        Self {
            exec,
            busy: Arc::new(Mutex::new(()))
        }
    }
}

async fn execute(exec: &Executor, stops: Vec<String>, prompt: String) -> String {
    let mut opts = Options::builder();
    opts.add_option(Opt::StopSequence(stops));
    let opts = opts.build();
    let prompt = Prompt::Text(
        prompt
        //&format!("\n{}: {}\nBot:", msg.author.name, msg.content)
    );
    let res = exec.execute(
        &opts,
        &prompt
    ).await.unwrap();
    res.to_immediate().await.unwrap().to_string()
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _:Context, ready:Ready) {
        println!("{} is connected", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache) || !msg.mentions_me(&ctx.http).await.unwrap() {
            return
        }

        let _busy = match self.busy.try_lock() {
            Ok(b) => b,
            Err(_) => {
                msg.reply(&ctx.http, "❌ Sorry, I'm already busy with something else. I can handle only one request at a time. Come back later").await.unwrap();
                return
            }
        };

        let reply = msg.reply(&ctx.http, "🔄 Thinking... (This can take a lot of time so be patient)").await.unwrap();

        let content = msg.content_safe(&ctx.cache);
        let content = match ctx.cache.current_user().discriminator {
            Some(d) => {
                let mut d = d.to_string();
                while d.chars().count() < 4 {
                    d.insert(0, '0');
                }
                content.trim_start_matches(&dbg!(format!("@{}#{}", ctx.cache.current_user().name, d)))
            },
            None => content.trim_start_matches(&dbg!(format!("@{}", ctx.cache.current_user().name)))
        };
        let content = content.trim();

        let prompt = format!(
            "Below is an instruction that describes a task. Write a response that appropriately completes the request.\n\n### Instruction:\n{}\n\n### Response:\n", 
            content
        );

        println!("Responding to {}…", msg.author.name);
        let res = execute(
            &self.exec,
            vec![],
            prompt
        ).await;
        dbg!(&res);
        let res = res.trim();

        if let Err(e) = reply.delete(&ctx.http).await {
            eprintln!("WARN: Cannot delete \"thinking\" message :\n- message id : {}\n- error : {}", reply.id, e);
        }

        msg.reply_ping(&ctx.http, res).await.unwrap();
        drop(_busy);
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
