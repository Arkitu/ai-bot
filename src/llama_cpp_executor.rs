use futures::executor::block_on;
use llm_chain::{traits::{Executor as ExecutorTrait, ExecutorCreationError, ExecutorError}, options::{Options, OptDiscriminants, Opt}, prompt::{Prompt, Data, ChatRole}, output::Output, tokens::{TokenizerError, Tokenizer as TokenizerTrait, TokenCount, PromptTokensError}};
use serenity::async_trait;

const MAX_TOKENS: usize = 128;

pub struct Executor {
    http: reqwest::Client,
    /// /!\ With no / at the end. ex: "https://127.0.0.1:8080"
    adress: String
}

mod req_res {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize)]
    pub struct ReqCompletion<'a> {
        pub prompt: &'a str,
        pub stop: &'a Vec<String>
    }

    #[derive(Deserialize)]
    pub struct ResCompletion {
        pub content: String
    }

    #[derive(Serialize)]
    pub struct ReqTokenize<'a> {
        pub content: &'a str
    }

    #[derive(Deserialize)]
    pub struct ResTokenize {
        pub tokens: Vec<usize>
    }

    #[derive(Serialize)]
    pub struct ReqDetokenize {
        pub tokens: Vec<usize>
    }

    #[derive(Deserialize)]
    pub struct ResDetokenize {
        pub content: String
    }
}
use req_res::*;

#[async_trait]
impl ExecutorTrait for Executor {
    type StepTokenizer<'a> = Tokenizer<'a>;

    fn new_with_options(_: Options) -> Result<Self, ExecutorCreationError> {
        Ok(Self {
            http: reqwest::Client::new(),
            adress: "http://127.0.0.1:8080".to_string()
        })
    }

    async fn execute(&self, opts: &Options, prompt: &Prompt) -> Result<Output, ExecutorError> {
        let prompt = prompt.to_text();
        let v = Vec::new();
        let stops = match opts.get(OptDiscriminants::StopSequence) {
            None => &v,
            Some(opt) => if let Opt::StopSequence(stops) = opt {
                stops
            } else {
                panic!()
            }
        };
        dbg!(&prompt);
        let res = self.http.post(self.adress.clone() + "/completion").json(&ReqCompletion {
            prompt: &prompt,
            stop: stops
        }).send().await.unwrap();
        let res: ResCompletion = res.json().await.unwrap();

        Ok(Output::new_immediate(Data::Text(res.content)))
    }

    fn tokens_used(
        &self,
        options: &Options,
        prompt: &Prompt,
    ) -> Result<TokenCount, PromptTokensError> {
        let tokenizer = self.get_tokenizer(options)?;
        let input = prompt.to_text();
        let mut tokens_used = tokenizer
            .tokenize_str(&input)
            .map_err(|_e| PromptTokensError::UnableToCompute)?
            .len() as i32;
        // includes answer_prefix
        let answer_prefix = self.answer_prefix(prompt);
        if let Some(prefix) = answer_prefix {
            let answer_used = tokenizer
                .tokenize_str(&prefix)
                .map_err(|_e| PromptTokensError::UnableToCompute)?
                .len() as i32;
            tokens_used += answer_used
        }
        let max_tokens = self.max_tokens_allowed(options);
        Ok(TokenCount::new(max_tokens, tokens_used))
    }

    fn answer_prefix(&self, prompt: &Prompt) -> Option<String> {
        if let llm_chain::prompt::Data::Chat(_) = prompt {
            // Tokenize answer prefix
            // XXX: Make the format dynamic
            let prefix = if prompt.to_text().ends_with('\n') {
                ""
            } else {
                "\n"
            };
            Some(format!("{}{}:", prefix, ChatRole::Assistant))
        } else {
            None
        }
    }

    fn max_tokens_allowed(&self, _: &Options) -> i32 {
        MAX_TOKENS as i32
    }

    fn get_tokenizer(&self, _: &Options) -> Result<Tokenizer, TokenizerError> {
        Ok(Tokenizer { exec: self })
    }
}

pub struct Tokenizer<'a> {
    exec: &'a Executor
}

impl<'a> TokenizerTrait for Tokenizer<'a> {
    fn tokenize_str(&self, doc: &str) -> Result<llm_chain::tokens::TokenCollection, TokenizerError> {
        block_on(async move {
            let res = self.exec.http.post(self.exec.adress.clone() + "/tokenize").json(&ReqTokenize {
                content: &doc.to_owned()
            }).send().await.map_err(|_|TokenizerError::TokenizationError)?;
            let res: ResTokenize = res.json().await.map_err(|_|TokenizerError::TokenizationError)?;
            Ok(res.tokens.into())
        })
    }

    fn to_string(&self, tokens: llm_chain::tokens::TokenCollection) -> Result<String, TokenizerError> {
        block_on(async move {
            let res = self.exec.http.post(self.exec.adress.clone() + "/detokenize").json(&ReqDetokenize {
                tokens: tokens.as_usize()?
            }).send().await.map_err(|_|TokenizerError::TokenizationError)?;
            let res: ResDetokenize = res.json().await.map_err(|_|TokenizerError::TokenizationError)?;
            Ok(res.content)
        })
    }
}