use config::{Config, ConfigError, Environment, File};
use log::{debug, log_enabled, Level};
use serde_derive::Deserialize;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    pub ai_settings: AiSettings,
    pub git_settings: GitSettings,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AiSettings {
    pub api_key: String,
    pub api_url: String,
    pub ai_options: AiOptions,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AiOptions {
    /// model name
    pub model: String,
    /// The maximum number of tokens to generate in the completion.
    /// The token count of your prompt plus max_tokens cannot exceed the model's context length.
    /// Most models have a context length of 2048 tokens (except for the newest models, which support 4096).
    pub max_tokens: u16,
    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will
    /// make the output more random, while lower values
    /// like 0.2 will make it more focused and deterministic.
    /// **NOTE:** Its reccomended to this this or `top_p` but not both.
    pub temperature: f32,
    /// An alternative to sampling with temperature,
    /// called nucleus sampling, where the model considers the results of the
    /// tokens with top_p probability mass. So 0.1 means only the tokens
    /// comprising the top 10% probability mass are considered.
    /// **NOTE:** Its reccomended to this this or `temperature` but not both.
    pub top_p: f32,
    /// How many completions to generate for each prompt.
    /// **NOTE:** This eats up your token allotment pretty quickly
    pub n: u8,
    /// Include the log probabilities on the logprobs most likely tokens, as well the chosen tokens.
    /// For example, if logprobs is 5, the API will return a list of the 5 most likely tokens.
    /// The API will always return the logprob of the sampled token,
    /// so there may be up to logprobs+1 elements in the response.
    pub logprobs: u8,
    /// Echo back the prompt in addition to the completion
    pub echo: bool,
    //// Up to 4 sequences where the API will stop generating further tokens.
    /// The returned text will not contain the stop sequence.
    pub stop: Vec<String>,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far, 
    /// increasing the model's likelihood to talk about new topics.
    pub presence_penalty: f32,
    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far, 
    /// decreasing the model's likelihood to repeat the same line verbatim.
    pub frequency_penalty: f32,
    /// Generates best_of completions server-side and returns the "best" (the one with the highest log probability per token). 
    /// Results cannot be streamed.
    /// When used with n, best_of controls the number of candidate completions and n specifies how many to return
    ///  – best_of must be greater than n.
    /// **NOTE:** This is a real token burner
    pub best_of: u8,
    /// The prompt(s) to generate completions for
    pub prompt: AiPrompt,
    /// turn auto-ai accept mode on
    pub auto_ai: bool,
    /// turn stocastic mode on
    pub stochastic: bool,
}

/// Default implementation, the defaults here **EXCEPT** for prompt are pretty good. 
///  See `AiPrompt` for more info
impl Default for AiOptions {
    fn default() -> Self {
        AiOptions { 
            model: "code-davinci-00".to_string(), 
            max_tokens: 256, 
            temperature: 0.05, 
            top_p: 1.0, 
            n: 1, 
            logprobs: 0, 
            echo: false, 
            stop: vec!["".into()], 
            presence_penalty: 0.1, 
            frequency_penalty: 0.1, 
            best_of: 1, 
            prompt: AiPrompt::default(), 
            auto_ai: false, 
            stochastic: false 
        }
    }
}


#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AiPrompt {
    /// The preamble (everything before the language) for the prompt
    pub preamble: String,
    /// The language **Please note this defaults to `python` if you dont change it
    pub language: String,
    /// Anything after the language and before the diff
    pub postamble: String,
    /// char that acts as a separator for the git diff, defaults to '='
    pub seperator: char,
    /// the actual git diff to analyze, this defaults to a silly python script
    pub git_diff: String,
    /// anything after the git diff
    pub postmessage: String,
}
/// default implememtation of our prompt to send to OpenAi
/// **NOTE** `language` amd `git_diff` should be changed from their default values
impl Default for AiPrompt {
    fn default() -> Self {
        AiPrompt { 
            preamble: "Imagine you are the most experianced ".to_string(), 
            language: "python ".to_string(),
            postamble: "in the world.  You were just handed the below Git Diff file to review.  Please summarize the changes encoded in the Git Diff".to_string(),
            seperator: '-', 
            git_diff: DEFAULT_CODE.to_string(),
            postmessage: "Please limit yourself to one paragraph".to_string()
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct GitSettings {
    pub github_api_key: String,
    pub github_api_url: String,
    pub git_options: GitOptions,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct GitOptions {
    pub local_path: PathBuf,
    pub auto_add: bool,
    pub auto_push: bool,
    pub sign_commits: bool,
    pub key_id: String,
    pub git_user_name: String,
    pub git_user_email: String,
    pub ssh_key_path: String,
    pub ssh_user_name: String,
}

impl Default for GitOptions {
    fn default() -> Self {
        GitOptions { local_path: PathBuf::from_str("."), auto_add: false, auto_push: true, sign_commits: false, key_id: String::new(), git_user_name: String::new(), git_user_email: String::new(), ssh_key_path: String::new(), ssh_user_name: () }
    }
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(File::with_name("~/.gitai/settings.json").required(true))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(
                Environment::with_prefix("gitai")
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(" "),
            )
            // You may also programmatically change settings
            .build()?;
        // You can deserialize (and thus freeze) the entire configuration as
        if log_enabled!(Level::Debug) {
            debug!("Settings are loaded.  Here they are");
            debug!("{:#?}", s);
        }
        s.try_deserialize()
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