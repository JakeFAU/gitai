use std::{collections::HashMap, path::PathBuf};

use crate::Value;
use git2::Repository;
use reqwest::Client;

#[derive(Debug)]
pub struct Git {
    local_path: PathBuf,
    client: Option<reqwest::blocking::Client>,
    git_remote_token: Option<String>,
    git_remote_url: Option<String>,
    auto_add: bool,
    auto_ai: bool,
}

impl Git {
    pub fn new(
        local_path: PathBuf,
        client: reqwest::blocking::Client,
        token: String,
        url: String,
        auto_add: Option<bool>,
        auto_ai: Option<bool>,
    ) -> Self {
        let git = Git {
            local_path: local_path,
            client: Some(client),
            git_remote_token: Some(token),
            git_remote_url: Some(url),
            auto_add: auto_add.unwrap_or(false),
            auto_ai: auto_ai.unwrap_or(false),
        };
        return git;
    }

    pub fn new_local_only(
        local_path: PathBuf,
        auto_add: Option<bool>,
        auto_ai: Option<bool>,
    ) -> Self {
        let git = Git {
            local_path: local_path,
            client: None,
            git_remote_token: None,
            git_remote_url: None,
            auto_add: auto_add.unwrap_or(false),
            auto_ai: auto_ai.unwrap_or(false),
        };
        return git;
    }

    pub fn open_repo(&self) -> Result<Repository, git2::Error> {
        let repo = git2::Repository::open(&self.local_path);
        return repo;
    }
}
