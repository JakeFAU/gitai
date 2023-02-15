use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use log::{debug, error};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Settings {
    pub ai_settings: AiSettings,
    pub git_settings: Option<GitSettings>,
}

impl Settings {
    pub fn from(path: PathBuf) -> Self {
        debug!("Checking {:#?} to see if it exists", &path);
        if path.exists() {
            debug!("{:#?} exists, reading settings", &path);
            let mut file = match File::open(path) {
                Ok(f) => f,
                Err(..) => panic!("Cannot read settings.json evem though it was found"),
            };
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Cannot read settings.json");
            let s: Settings = match from_reader(contents.as_bytes()) {
                Ok(s) => s,
                Err(..) => {
                    error!("Your settings.json file is of an invalid format.  Here is a schema to help:\n");
                    let schema = schema_for!(Settings);
                    error!("{}", serde_json::to_string_pretty(&schema).unwrap());
                    panic!("Have a great day")
                }
            };
            return s;
        } else {
            debug!("{:#?} doesn't exist", &path);
            debug!("We will put a json schema file in $HOME/.gitai so you can create one");
            let schema = schema_for!(Settings);
            let contents = format!("{}", serde_json::to_string_pretty(&schema).unwrap());
            fs::write("~/.gitai/settings.json", contents).expect("Unable to write file");
            panic!("Have a great day!");
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiSettings {
    pub api_key: String,
    pub api_url: String,
    pub ai_options: Option<AiOptions>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiOptions {
    pub model: Option<String>,
    pub suffix: Option<String>,
    pub max_tokens: Option<u16>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub n: Option<u8>,
    pub logprobs: Option<u8>,
    pub echo: Option<bool>,
    pub stop: Option<String>,
    pub presence_penalty: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub best_of: Option<u8>,
    pub prompt: Option<AiPrompt>,
    pub auto_ai: Option<bool>,
    pub stochastic: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AiPrompt {
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

impl AiOptions {
    fn is_valid(&self) -> bool {
        let mut b =
            self.max_tokens.unwrap_or_default() > 0 && self.max_tokens.unwrap_or_default() <= 4096;
        b = b
            && self.temperature.unwrap_or_default() >= 0.0
            && self.temperature.unwrap_or_default() <= 2.0;
        b = b && self.top_p.unwrap_or_default() >= 0.0 && self.top_p.unwrap_or_default() <= 1.0;
        b = b && self.n.unwrap_or_default() >= 1 && self.n.unwrap_or_default() <= 3;
        b = b && self.logprobs.unwrap_or_default() <= 5;
        b = b
            && self.presence_penalty.unwrap_or_default() >= -2.0
            && self.presence_penalty.unwrap_or_default() <= 2.0;
        b = b
            && self.frequency_penalty.unwrap_or_default() >= -2.0
            && self.frequency_penalty.unwrap_or_default() <= 2.0;
        return b;
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitSettings {
    pub github_api_key: Option<String>,
    pub github_api_url: Option<String>,
    pub git_options: Option<GitOptions>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitOptions {
    pub local_path: Option<String>,
    pub auto_add: Option<bool>,
    pub auto_push: Option<bool>,
    pub sign_commits: Option<bool>,
    pub key_id: Option<String>,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
    pub ssh_key_path: Option<String>,
    pub ssh_user_name: Option<String>,
}
