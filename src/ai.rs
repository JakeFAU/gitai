use std::{collections::HashMap, str::FromStr};

use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, info};
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

// The request params to send to OpenAi for or completion
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenAiRequestParams {
    /// The Open AI Model to use
    pub model: String,
    /// The prompt to send to Open AI
    pub prompt: String,
    /// Anything after the prompt that should be sent to Open AI
    pub suffix: Option<String>,
    /// Max Tokens - Note: this is how long the output can be, and will effect your bill
    pub max_tokens: Option<u16>,
    /// Temperature to pass to the model - Note: For code they reccomend a value near 0
    pub temperature: Option<f32>,
    /// nucleus sampling - Note: They reccomend only setting one of this or temperature, not both
    pub top_p: Option<f32>,
    /// number of completions to send back
    pub n: Option<u8>,
    /// The number of logprobs to return, defaults to 0
    pub logprobs: Option<u8>,
    /// Return the prompt
    pub echo: Option<bool>,
    /// a string that will stop the tokenizer at OpenAI from tokenizing
    pub stop: Option<String>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far
    pub presence_penalty: Option<f32>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far
    pub frequency_penalty: Option<f32>,
    /// Generates best_of completions server-side and returns the "best" (the one with the highest log probability per token).
    /// When used with n, best_of controls the number of candidate completions and n specifies how many to return –
    /// best_of must be greater than n.
    pub best_of: Option<u8>,
}
/// An OpenAiChoice is basically the answer.  If n>1 his can be a Vector
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiChoice {
    /// The response
    pub text: Option<String>,
    /// The index number of this choice
    pub index: Option<u8>,
    /// logprobs (if set to return)
    pub logprobs: Option<f32>,
    /// why the completion stopped
    pub finish_reason: Option<String>,
}

/// Shows you how many tokens you used on this request.  This affects your bill
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiUsage {
    /// The number of tokens in the prompt
    pub prompt_tokens: Option<u16>,
    /// The number of tokens in the completion
    pub completion_tokens: Option<u16>,
    /// The total number of tokens.  This is what you are billed for
    pub total_tokens: Option<u16>,
}
/// The response that comes back from OpenAI for a completion
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiCompletionResponse {
    /// An Id
    pub id: Option<String>,
    /// what OpenAi did (should be 'text_completion' for this)
    pub object: Option<String>,
    /// A timestamp of when this was created
    pub created: Option<u64>,
    /// which model to use, right now code-davinici-002 is the best
    pub model: Option<String>,
    /// The choices it returned, this will be a Vec whose length is equal to n for the request
    pub choices: Option<Vec<OpenAiChoice>>,
    /// The usage this request used
    pub usage: Option<OpenAiUsage>,
}

/// Default Implementation - Sets all things **except** the prompt to what you probably want to use
/// so be sure to create it mutable so you can set the prompt
impl Default for OpenAiRequestParams {
    fn default() -> Self {
        OpenAiRequestParams {
            model: String::from_str("code-davinci-002").expect("Why cant I set the default?"),
            prompt: String::from_str("Say hello to Jake for me")
                .expect("Why cant I set the default?"),
            suffix: None,
            max_tokens: Some(256),
            temperature: Some(0.05),
            top_p: Some(1.0),
            n: Some(1),
            logprobs: None,
            echo: Some(false),
            stop: None,
            presence_penalty: Some(0.2),
            frequency_penalty: Some(0.2),
            best_of: Some(1),
        }
    }
}

#[derive(Debug, Clone)]
/// A Client for commnicating with OpenAi
pub struct OpenAIClient {
    /// The base url for OpenAI's API
    pub base_url: Url,
    /// The api key used to make requestes to OpenAI
    api_key: String,
    ///A map of headers for easy reuse
    headers: HeaderMap,
}

impl OpenAIClient {
    pub fn new(api_key: &str, base_url: Option<Url>) -> Self {
        info!("Creating new OpenAI Client");
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        let url = match base_url {
            Some(u) => u,
            None => Url::parse("https://api.openai.com/v1/").unwrap(),
        };
        OpenAIClient {
            base_url: url,
            api_key: api_key.to_string(),
            headers: headers,
        }
    }

    pub fn get_models(&self) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
        info!("Getting Available Models");
        debug!("This is mainly useful to make sure you can talk to the OpenAi API");
        let url = self.base_url.join("models")?;
        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(self.headers.clone())
            .build()?;
        let response = client.get(url).bearer_auth(self.api_key.clone()).send()?;
        let json = response.json::<HashMap<String, Value>>()?;
        return Ok(json);
    }

    fn get_single_completion(
        &self,
        params: OpenAiRequestParams,
    ) -> Result<OpenAiCompletionResponse, Box<dyn std::error::Error>> {
        info!("Sending single request to OpenAi");
        let url = self.base_url.join("completions")?;
        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(self.headers.clone())
            .build()?;
        let response = client
            .post(url)
            .bearer_auth(self.api_key.clone())
            .json(&params)
            .send()?;
        let ai = response.json::<OpenAiCompletionResponse>()?;
        return Ok(ai);
    }

    async fn get_multiple_completions(
        &self,
        params: Vec<OpenAiRequestParams>,
    ) -> Result<Vec<OpenAiCompletionResponse>, Box<dyn std::error::Error>> {
        info!("Sending multiple requests to OpenAi");
        let url = self.base_url.join("completions")?;
        let client = reqwest::ClientBuilder::new()
            .default_headers(self.headers.clone())
            .build()?;
        let mut futs: FuturesUnordered<_> = FuturesUnordered::new();
        for param in params {
            let response = client
                .post(url.clone())
                .bearer_auth(self.api_key.clone())
                .json(&param)
                .send()
                .await?;
            let fut = response.json::<OpenAiCompletionResponse>();
            futs.push(fut);
        }
        let mut results = Vec::new();
        while let Some(result) = futs.next().await {
            results.push(result?);
        }
        Ok(results)
    }
}
