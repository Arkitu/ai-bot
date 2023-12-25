use llm_chain::options::{Options, Opt};
use llm_chain::prompt::Prompt;
use llm_chain::traits::Executor as ExecutorTrait;
use llm_chain::{prompt, parameters};
use serenity::all::ChannelId;
use serenity::builder::{CreateChannel, EditMessage, CreateEmbed, CreateEmbedAuthor};
use tokio;
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

mod llama_cpp_executor;
use llama_cpp_executor::Executor;

struct Handler {
    exec: Executor,
    chats: Arc<RwLock<HashMap<ChannelId, String>>>
}

impl Handler {
    pub fn new() -> Self {
        let exec = Executor::new().unwrap();
        Self {
            exec,
            chats: Arc::new(RwLock::new(HashMap::new()))
        }
    }
    pub async fn create_chat(&self, chan: ChannelId) {
        let mut chats = self.chats.write().await;

        chats.insert(chan, "".to_string());
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

        

        let mut reply = msg.reply(&ctx.http, "🔄 Réfléchit...").await.unwrap();

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

        let stops = vec![
            "SYSTEM:".to_string(),
            "USER:".to_string(),
            msg.author.name.clone() + ":"
        ];
        let prompt = format!("\
            SYSTEM: Tu es AI Bot, un bot discord Français.\
            Tu es sympa et utilise des emojis.\n\n\
            USER: {}\n\n\
            SYSTEM: Tu peux faire une des actions suivantes:\n\
            - ne rien faire
            - répondre\n\
            - créer un salon\n\
            - supprimer un salon\n\
            Tu dois l'écrire EXACTEMENT et ne rien ajouter
            ACTION: \
        ", content);

        println!("Responding to {}…", msg.author.name);
        let res = execute(
            &self.exec,
            stops,
            format!("\
                {}: {}\n\n\
                ASSISTANT: \
            ", msg.author.name, content)
        ).await;
        dbg!(&res);
        let res = res.trim();

        let avatar_url = ctx.cache.current_user().face();

        reply.edit(&ctx.http, EditMessage::new()
            .content("")
            .add_embed(
                CreateEmbed::new()
                    .author(CreateEmbedAuthor::new("AI Bot").icon_url(avatar_url))
                    .description(res)
            )
        ).await.unwrap();

        msg.react(&ctx.http, '✅').await.unwrap();
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
