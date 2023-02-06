use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug)]
pub struct AI {
    client: reqwest::blocking::Client,
    base_url: String,
    token: String,
}

impl AI {
    pub fn new(base_url: &str, token: &str) -> Self {
        let client = reqwest::blocking::Client::new();
        Self {
            client,
            base_url: String::from(base_url),
            token: String::from(token),
        }
    }

    pub fn get_models(self) -> HashMap<String, Value> {
        let mut url = self.base_url;
        url.push_str("models");
        let res = self.client.get(url).bearer_auth(&self.token).send();
        let contents = match res {
            Ok(r) => r.text().unwrap(),
            Err(..) => String::from(""),
        };
        let json = serde_json::from_str(&contents).unwrap();
        return json;
    }
}
