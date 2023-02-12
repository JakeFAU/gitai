use std::{collections::HashMap, str::FromStr};

use log::{debug, info};
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiRequestParams {
    model: String,
    prompt: String,
    suffix: Option<String>,
    max_tokens: Option<u16>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    n: Option<u8>,
    logprobs: Option<u8>,
    echo: Option<bool>,
    stop: Option<String>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    best_of: Option<u8>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiChoice {
    text: Option<String>,
    index: Option<u8>,
    logprobs: Option<f32>,
    finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiUsage {
    prompt_tokens: Option<u16>,
    completion_tokens: Option<u16>,
    total_tokens: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiCompletionResponse {
    id: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Option<Vec<OpenAiChoice>>,
    usage: Option<OpenAiUsage>,
}

impl Default for OpenAiRequestParams {
    fn default() -> Self {
        OpenAiRequestParams {
            model: String::from_str("code-davinci-002").expect("Why cant I set the default?"),
            prompt: String::from_str("Say hello to Jake for me")
                .expect("Why cant I set the default?"),
            suffix: None,
            max_tokens: Some(256),
            temperature: Some(0.0),
            top_p: Some(1.0),
            n: Some(1),
            logprobs: None,
            echo: Some(false),
            stop: None,
            presence_penalty: Some(0.0),
            frequency_penalty: Some(0.0),
            best_of: Some(1),
        }
    }
}

#[derive(Debug)]
pub struct OpenAiClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl OpenAiClient {
    pub fn new(base_url: String, open_api_token: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", open_api_token).parse().unwrap(),
        );
        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Error Building Reqwest Client");
        let ai_client = OpenAiClient {
            client: client,
            base_url: base_url,
        };
        return ai_client;
    }

    pub fn get_models(&self) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
        info!("Getting Available Models");
        let url = format!("{}models", self.base_url);
        debug!("url={:#?}", url);
        let res = self.client.get(url).send().unwrap();
        let jsn = res.json::<HashMap<String, Value>>()?;
        return Ok(jsn);
    }

    pub fn get_completions(
        &self,
        git_diff_text: String,
    ) -> Result<OpenAiCompletionResponse, Box<dyn std::error::Error>> {
        info!("Getting Completion");
        let url = format!("{}completions", self.base_url);
        debug!("url={:#?}", url);
        let mut request_params = OpenAiRequestParams::default();
        request_params.prompt = git_diff_text;
        let res = self.client.post(url).json(&request_params).send().unwrap();
        let data = res.json::<OpenAiCompletionResponse>().unwrap();
        return Ok(data);
    }
}
