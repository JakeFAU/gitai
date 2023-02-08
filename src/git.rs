use git2::{Diff, DiffFormat, DiffOptions, Error, IndexAddOption, Repository};
use log::debug;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

pub struct Git {
    local_path: PathBuf,
    github_token: Option<String>,
    github_url: Option<String>,

    // flags
    auto_add: bool,
    sign_commit: bool,
    key_id: String,
    key_signature: String,

    // stuff to set up
    client: Option<reqwest::blocking::Client>,
}

impl Git {
    // This is if you want to make PRs
    pub fn new(
        local_path: PathBuf,
        client: reqwest::blocking::Client,
        github_token: String,
        github_url: String,
        auto_add: Option<bool>,
        sign_commit: Option<bool>,
        key_id: Option<String>,
        key_signature: Option<String>,
    ) -> Self {
        let git = Git {
            local_path: local_path,
            client: Some(client),
            github_token: Some(github_token),
            github_url: Some(github_url),
            auto_add: auto_add.unwrap_or(false),
            sign_commit: sign_commit.unwrap_or(false),
            key_id: key_id.unwrap_or_default(),
            key_signature: key_signature.unwrap_or_default(),
        };
        return git;
    }

    // This is for commit only
    pub fn new_local_only(
        local_path: PathBuf,
        auto_add: Option<bool>,
        sign_commit: Option<bool>,
        key_id: Option<String>,
        key_signature: Option<String>,
    ) -> Self {
        let git = Git {
            local_path: local_path,
            client: None,
            github_token: None,
            github_url: None,
            auto_add: auto_add.unwrap_or(false),
            sign_commit: sign_commit.unwrap_or(false),
            key_id: key_id.unwrap_or_default(),
            key_signature: key_signature.unwrap_or_default(),
        };
        return git;
    }

    fn _add_files_to_index(self, repo: &Repository) -> Result<(), Error> {
        debug!("Adding untracked files to index");
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn get_diff(self) {
        debug!("Opening Repo");
        let repo = git2::Repository::open(&self.local_path).expect("Cannot open repo");
        // If auto add is set, call the add function
        debug!("Checking auto-add");
        debug!("auto_add = {:#?}", &self.auto_add);
        if self.auto_add {
            self._add_files_to_index(&repo)
                .expect("Error Adding New Files To The Index");
        }
        let head = &repo.head().expect("Cannot get HEAD");
        let oid = &head.target().unwrap();
        let commit = &repo.find_commit(*oid).expect("Cannot Find HEAD commit");
        let old_tree = &commit.tree().expect("Error getting HEAD tree");
        let index = &repo.index().expect("Error getting Index");
        let diff = repo
            .diff_tree_to_index(
                Some(&old_tree),
                Some(&index),
                Some(&mut DiffOptions::default()),
            )
            .expect("Unable to get DIFF");
        diff.print(DiffFormat::Raw, true);
    }
}
