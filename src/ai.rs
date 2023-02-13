use std::{
    cmp::min,
    collections::HashMap,
    fmt::{self, Display},
    iter::repeat,
    str::FromStr,
};

use log::{debug, info};
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// This struct constructs the prompt to send to OpenAi
/// The default implementation has good values for everything but
/// `language`, `git_diff` and `postmessage` although the `postmessage`
/// isn't really that important
#[derive(Debug)]
pub struct Prompt {
    /// The preamble (everything before the language) for the prompt
    pub preamble: Option<String>,
    /// The language **Please note this defaults to `python` if you dont change it
    pub language: Option<String>,
    /// Anything after the language and before the diff
    pub postamble: Option<String>,
    /// char that acts as a separator for the git diff, defaults to '='
    pub seperator: Option<char>,
    /// the actual git diff to analyze, this defaults to a silly python script
    pub git_diff: Option<String>,
    /// anything after the git diff
    pub postmessage: Option<String>,
}

/// Default implementation of the prompt
impl Default for Prompt {
    fn default() -> Self {
        Prompt {
            preamble: Some("Imagine you are an expert ".to_string()),
            language: Some("Python  ".to_string()),
            postamble: Some("developer and were given a git diff file to look at:".to_string()),
            git_diff: Some(DEFAULT_CODE.to_string()),
            seperator: Some('-'),
            postmessage: Some(
                "Please generate a good explanation of what the developer did. Stop when you think its clear enough to move on.".to_string(),
            ),
        }
    }
}

/// Display information for the prompt
impl Display for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}\n{}\n{}\n{}\n{}",
            self.preamble.as_ref().unwrap_or(&"".to_string()),
            self.language.as_ref().unwrap_or(&"".to_string()),
            self.postamble.as_ref().unwrap_or(&"".to_string()),
            repeat(self.seperator.unwrap_or('-'))
                .take(16)
                .collect::<String>(),
            self.git_diff.as_ref().unwrap_or(&"".to_string()),
            repeat(self.seperator.unwrap_or('-'))
                .take(16)
                .collect::<String>(),
            self.postmessage.as_ref().unwrap_or(&"".to_string()),
        )
    }
}
// The request params to send to OpenAi for or completion
#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAiRequestParams {
    /// The Open AI Model to use
    model: String,
    /// The prompt to send to Open AI
    prompt: String,
    /// Anything after the prompt that should be sent to Open AI
    suffix: Option<String>,
    /// Max Tokens - Note: this is how long the output can be, and will effect your bill
    max_tokens: Option<u16>,
    /// Temperature to pass to the model - Note: For code they reccomend a value near 0
    temperature: Option<f32>,
    /// nucleus sampling - Note: They reccomend only setting one of this or temperature, not both
    top_p: Option<f32>,
    /// number of completions to send back - TODO: Implement this as an aysnc for now it does nothing
    n: Option<u8>,
    /// The number of logprobs to return, defaults to 0
    logprobs: Option<u8>,
    /// Return the prompt
    echo: Option<bool>,
    /// a string that will stop the tokenizer at OpenAI from tokenizing
    stop: Option<String>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far
    presence_penalty: Option<f32>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far
    frequency_penalty: Option<f32>,
    /// Generates best_of completions server-side and returns the "best" (the one with the highest log probability per token).
    /// When used with n, best_of controls the number of candidate completions and n specifies how many to return –
    /// best_of must be greater than n.
    best_of: Option<u8>,
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

/// A simple little client for making requests to OpenAi
#[derive(Debug)]
pub struct OpenAiClient {
    /// The reqwest client - TODO: Make this a non-blocking one
    client: reqwest::blocking::Client,
    /// The base url for the OpenApi API
    base_url: String,
}

impl OpenAiClient {
    /// Returns an OpenAiClient with the base url and api token
    ///
    /// # Arguments
    ///
    /// * `base_url` - A string containing the base url for the API
    /// * `open_api_token` - The OpenAi token to use
    ///
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

    /// Gets all the models available at OpenAi - THis is mainly to test
    /// if your token is valid
    ///
    /// Returns `Ok(HashMap<String, Value>)` on success, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// This method fails whenever the response body is not in JSON format or it
    /// cannot be properly deserialized to target type T.
    /// For more details please see [serde_json::from_reader](https://docs.serde.rs/serde_json/fn.from_reader.html)
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    ///
    pub fn get_models(&self) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
        info!("Getting Available Models");
        let url = format!("{}models", self.base_url);
        debug!("url={:#?}", url);
        let res = self.client.get(url).send()?;
        let jsn = res.json::<HashMap<String, Value>>()?;
        return Ok(jsn);
    }

    /// Gets the completions from a given Git Diff file
    ///
    /// # Arguments
    ///
    /// * `git_diff_text` - A string containing the text of the Git Diff Message
    /// * `open_ai_request_params` - Optional - will be set to sensible defaults and then the prompt changed to the git diff
    ///
    /// Returns `Ok(OpenAiCompletionResponse)` on success, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// This method fails whenever the response body is not in JSON format or it
    /// cannot be properly deserialized to target type T.
    /// For more details please see [serde_json::from_reader](https://docs.serde.rs/serde_json/fn.from_reader.html).
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    ///
    pub fn get_completions(
        &self,
        prompt: Prompt,
        open_ai_request_params: Option<OpenAiRequestParams>,
    ) -> Result<OpenAiCompletionResponse, Box<dyn std::error::Error>> {
        info!("Getting Completion");
        let url = format!("{}completions", self.base_url);
        debug!("url={:#?}", url);
        let mut request_params = open_ai_request_params.unwrap_or_default();
        request_params.prompt = format!("{}", prompt);
        request_params.max_tokens = Some(min(
            <usize as TryInto<u16>>::try_into(request_params.prompt.chars().count()).unwrap() / 10,
            256,
        ));
        debug!("Max Tokens Set To {}", &request_params.max_tokens.unwrap());
        let res = self.client.post(url).json(&request_params).send()?;
        let data = res.json::<OpenAiCompletionResponse>()?;
        return Ok(data);
    }
}

const DEFAULT_CODE: &str = "
diff --git a/foo.py b/foo.py\n
new file mode 100644\n
index 0000000..e5a8e79\n
--- /dev/null\n
+++ b/foo.py\n
@@ -0,0 +1,5 @@\n
+def say_hi(name: str) -> str:\n
+    print(f'Hi {name}')\n
+\n
+if __name__ == 'main':\n
";
