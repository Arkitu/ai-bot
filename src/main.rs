use llm_chain::options::{Options, Opt};
use llm_chain::prompt::Prompt;
use llm_chain::traits::Executor as ExecutorTrait;
use llm_chain::{prompt, parameters};
use serenity::builder::{CreateChannel, EditMessage, CreateEmbed, CreateEmbedAuthor};
use tokio;
use dotenv::dotenv;
use std::env;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

mod llama_cpp_executor;
use llama_cpp_executor::Executor;

struct Handler {
    exec: Executor
}

impl Handler {
    pub fn new() -> Self {
        // let opts = options!(
        //     Model: ModelRef::from_path("./ggml-alpaca-7b-q4.bin"), // Notice that we reference the model binary path
        //     ModelType: "llama",
        //     MaxContextSize: 512_usize,
        //     NThreads: 4_usize,
        //     MaxTokens: 0_usize,
        //     TopK: 40_i32,
        //     TopP: 0.95,
        //     TfsZ: 1.0,
        //     TypicalP: 1.0,
        //     Temperature: 0.8,
        //     RepeatPenalty: 1.1,
        //     RepeatPenaltyLastN: 64_usize,
        //     FrequencyPenalty: 0.0,
        //     PresencePenalty: 0.0,
        //     Mirostat: 0_i32,
        //     MirostatTau: 5.0,
        //     MirostatEta: 0.1,
        //     PenalizeNl: true,
        //     StopSequence: vec!["\n".to_string()]
        // );
        // let exec = executor!(llama, opts).unwrap();
        let exec = Executor::new().unwrap();
        Self {
            exec
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
            "USER:".to_string()
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

        // let prompt = "\
        //     SYSTEM: Tu es AI Bot, un bot discord Français.\
        //     Tu es sympa et utilise des emojis.\
        //     Tu peux utliser les commandes suivantes en les envoyant dans le chat :\n\
        //     - /create_channel \"nom du salon\"\n\
        //     Par exemple : \"ASSISTANT: /create_channel general\" crée le salon general\n\
        //     - /delete_channel \"nom du salon\"\n\
        //     ".to_string() + &format!("USER: {}\nASSISTANT:", content);

        println!("Responding to {}…", msg.author.name);
        let res = execute(
            &self.exec,
            vec![
                "\n".to_string()
            ],
            format!("\
                USER: {}\n\n\
                SYSTEM: Tu peux faire une des actions suivantes:\n\
                /répondre\n\
                /créer un salon\n\
                /supprimer un salon\n\
                /ne rien faire\n\
                Tu dois l'écrire EXACTEMENT et ne rien ajouter\n\n\
                ASSISTANT: \
            ", content)
        ).await;
        dbg!(&res);
        let res = res.trim();

        if res.starts_with("/create_channel ") {
            let res = res.split_at(16).1;
            let res = res.trim_matches('"');
            if let Some(guild_id) = msg.guild_id {
                match guild_id.create_channel(&ctx.http, CreateChannel::new(res)).await {
                    Err(e) => {
                        reply.edit(&ctx.http, EditMessage::new().content(format!("❌ Erreur : {}", e))).await.unwrap();
                    },
                    Ok(chan) => {
                        reply.edit(&ctx.http, EditMessage::new().content(format!("✅ <#{}> créé !", chan.id))).await.unwrap();
                    }
                }
            } else {
                reply.edit(&ctx.http, EditMessage::new().content("❌ Erreur : j'ai essayé de créer un salon mais je ne suis pas dans un serveur")).await.unwrap();
            }
        } else {
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
