use llm_chain::options::{Options, Opt};
use llm_chain::prompt::Prompt;
use llm_chain::traits::Executor as ExecutorTrait;
use serenity::all::{ChannelId, Interaction, CommandDataOption, CommandDataOptionValue, Channel};
use serenity::builder::{EditMessage, CreateEmbed, CreateEmbedAuthor, CreateInteractionResponse, CreateInteractionResponseMessage, GetMessages, Builder};
use tokio;
use dotenv::dotenv;
use std::collections::HashMap;
use std::{env, vec};
use std::sync::Arc;
use serenity::prelude::*;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::async_trait;

mod llama_cpp_executor;
use llama_cpp_executor::Executor;

#[derive(PartialEq)]
enum ChatTrigger {
    OnMessage,
    OnMention
}
#[derive(PartialEq)]
enum ChatPrivacy {
    AllMessages,
    OnlyMentions
}

struct Chat {
    trigger: ChatTrigger,
    privacy: ChatPrivacy
}

struct Handler {
    exec: Executor,
    chats: Arc<RwLock<HashMap<ChannelId, Chat>>>
}

impl Handler {
    pub fn new() -> Self {
        let exec = Executor::new().unwrap();
        Self {
            exec,
            chats: Arc::new(RwLock::new(HashMap::new()))
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

struct CmdCreateChatOpts {
    trigger: ChatTrigger,
    privacy: ChatPrivacy,
    channel: ChannelId
}
impl CmdCreateChatOpts {
    fn new(chan: ChannelId) -> Self {
        Self {
            trigger: ChatTrigger::OnMessage,
            privacy: ChatPrivacy::AllMessages,
            channel: chan
        }
    }
    
    fn parse(cmd_opts: &Vec<CommandDataOption>, chan: ChannelId) -> Result<Self, String> {
        let mut opts = Self::new(chan);
        for opt in cmd_opts {
            match opt.name.as_str() {
                "trigger" => {
                    if let CommandDataOptionValue::String(val) = &opt.value {
                        opts.trigger = match val.as_str() {
                            "on_message" => ChatTrigger::OnMessage,
                            "on_mention" => ChatTrigger::OnMention,
                            _ => panic!("Unknown option")
                        }
                    } else {
                        panic!("Option value is not a string")
                    }
                },
                "privacy" => {
                    if let CommandDataOptionValue::String(val) = &opt.value {
                        opts.privacy = match val.as_str() {
                            "all_messages" => ChatPrivacy::AllMessages,
                            "only_mentions" => ChatPrivacy::OnlyMentions,
                            _ => panic!("Unknown option")
                        }
                    } else {
                        panic!("Option value is not a string")
                    }
                },
                "channel" => {
                    if let CommandDataOptionValue::Channel(val) = &opt.value {
                        opts.channel = *val;
                    } else {
                        panic!("Option value is not a channel")
                    }
                },
                _ => {}
            }
        }
        if opts.trigger == ChatTrigger::OnMessage && opts.privacy == ChatPrivacy::OnlyMentions {
            return Err("Vous ne pouvez pas créer un sallon avec les options `trigger=on_message` et `privacy=only_mentions`".to_string())
        }
        Ok(opts)
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _:Context, ready:Ready) {
        println!("{} is connected", ready.user.name);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            match cmd.data.name.as_str() {
                "create_chat" => {
                    let mut chats = self.chats.write().await;
                    if chats.contains_key(&cmd.channel_id) {
                        cmd.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content("Un chat existe déjà dans ce salon")
                        )).await.unwrap();
                        return
                    }
                    let chan = cmd.channel_id;
                    let opts = match CmdCreateChatOpts::parse(&cmd.data.options, chan) {
                        Ok(opts) => opts,
                        Err(err) => {
                            cmd.create_response(&ctx.http, CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::default()
                                    .content(err)
                            )).await.unwrap();
                            return
                        }
                    };
                    chats.insert(chan, Chat {
                        trigger: opts.trigger,
                        privacy: opts.privacy
                    });
                    cmd.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content(format!("Chat créé dans <#{}>", chan))
                    )).await.unwrap();
                }
                _ => {}
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx.cache) {
            return
        }

        let chats = self.chats.read().await;
        let chat = match chats.get(&msg.channel_id) {
            Some(chat) => chat,
            None => return
        };

        if chat.trigger == ChatTrigger::OnMention && !msg.mentions_me(&ctx.http).await.unwrap() {
            return
        }

        let discussion = match msg.channel(&ctx.http).await.unwrap() {
            Channel::Guild(chan) => {
                chan.messages(&ctx.http, GetMessages::new()).await
            },
            Channel::Private(chan) => {
                chan.messages(&ctx.http, GetMessages::new()).await
            },
            _ => panic!()
        }.unwrap();

        let mut reply = msg.reply(&ctx.http, "🔄 Réfléchit...").await.unwrap();

        let prompt = "<s>".to_string() +
            &discussion.iter().rev().map(|msg|{
                format!("[INST] {}: {} [/INST]", msg.author.name, msg.content_safe(&ctx.cache))
            }).collect::<Vec<_>>().join("</s>");

        // let content = msg.content_safe(&ctx.cache);
        // let content = match ctx.cache.current_user().discriminator {
        //     Some(d) => {
        //         let mut d = d.to_string();
        //         while d.chars().count() < 4 {
        //             d.insert(0, '0');
        //         }
        //         content.trim_start_matches(&dbg!(format!("@{}#{}", ctx.cache.current_user().name, d)))
        //     },
        //     None => content.trim_start_matches(&dbg!(format!("@{}", ctx.cache.current_user().name)))
        // };
        // let content = content.trim();

        println!("Responding to {}…", msg.author.name);
        let res = execute(
            &self.exec,
            vec![],
            //format!("<s>[INST] You are a Discord chatbot [/INST]</s>[INST]{}[/INST]", content)
            prompt
        ).await;
        dbg!(&res);
        let res = res.trim();

        let avatar_url = ctx.cache.current_user().face();

        reply.edit(&ctx.http, EditMessage::new()
            .content(res)
            // .add_embed(
            //     CreateEmbed::new()
            //         .author(CreateEmbedAuthor::new("AI Bot").icon_url(avatar_url))
            //         .description(res)
            // )
        ).await.unwrap();
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
