use llm_chain::{traits::{Executor as ExecutorTrait, ExecutorCreationError, ExecutorError}, options::Options, prompt::{Prompt, Data}, output::{Output, Immediate}, tokens::TokenizerError};
use serde::{Serialize, Deserialize};
use serenity::async_trait;

struct Executor {
    http: reqwest::Client,
    /// /!\ With no / at the end. ex: "https://127.0.0.1:8080"
    adress: String
}

#[derive(Serialize)]
struct ReqCompletion<'a> {
    prompt: &'a str
}

#[derive(Deserialize)]
struct ResCompletion {
    content: String
}

//#[async_trait]
impl ExecutorTrait for Executor {
    type StepTokenizer<'a> = Tokenizer<'a>;

    fn new_with_options(options: Options) -> Result<Self, ExecutorCreationError> {
        Ok(Self {
            http: reqwest::Client::new(),
            adress: "https://127.0.0.1:8080".to_string()
        })
    }

    async fn execute(&self, options: &Options, prompt: &Prompt) -> Result<Output, ExecutorError> {
        let prompt = prompt.to_text();
        let res = self.http.post(self.adress.clone() + "/completion").json(&ReqCompletion {
            prompt: &prompt
        }).send().await.unwrap();
        let res: ResCompletion = res.json().await.unwrap();

        Ok(Output::new_immediate(Data::Text(res.content)))
    }

    fn get_tokenizer(&self,options: &Options) -> Result<Tokenizer, TokenizerError> {
        Ok(Tokenizer { exec: self })
    }
}

struct Tokenizer<'a> {
    exec: &'a Executor
}