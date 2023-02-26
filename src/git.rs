use std::{
    collections::HashMap,
    path::{PathBuf, MAIN_SEPARATOR},
};

use git2::{
    Commit, Cred, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, IndexAddOption,
    ObjectType, Oid, PushOptions, RemoteCallbacks, Repository, Signature,
};
use log::{debug, info, log_enabled, Level};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde::Serialize;

/// Struct to hold information for your local Git
#[derive(Debug, Copy, Clone)]
pub struct Git<'a> {
    /// The path, should be '.' unless you have good reason
    pub path: &'a str,
    /// Should all untracked files be added to the index.  Basically the same as running `git add .` defaults to false
    pub auto_add: Option<&'a bool>,
    /// Should your local branch be pushed before creating a PR, defaults to true
    pub auto_push: Option<&'a bool>,
    /// Should commits be pgp signed - will look in git config if None
    pub sign_commits: Option<&'a bool>,
    /// The signing key id, this only matters if `sign_commits` is true - will look in git config if None
    pub key_id: Option<&'a str>,
    /// The git user name - will look in git config if None
    pub user_name: Option<&'a str>,
    /// The git user email - will look in git config if None
    pub user_email: Option<&'a str>,
    /// The path to the private key, will default to `$HOME/.ssh/id_rsa`
    pub ssh_key_path: Option<&'a str>,
    /// The ssh user name, i have never seen where it wasn't git
    pub ssh_user_name: Option<&'a str>,
}

/// Default implementation of the Git Opyions
impl Default for Git<'_> {
    fn default() -> Self {
        Git {
            path: ".",
            auto_add: Some(&false),
            auto_push: Some(&true),
            sign_commits: Some(&false),
            key_id: None,
            user_name: None,
            user_email: None,
            ssh_key_path: Some(&"~/.ssh/id_rsa"),
            ssh_user_name: Some(&"git"),
        }
    }
}

/// GitGub Options
#[derive(Debug, Default)]
pub struct GitHub {
    /// The GitHub API Token
    github_token: String,
    /// The GitHub API URL
    github_url: String,
    /// the GitHub user name
    github_username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PullResponse {
    url: String,
    html_url: String,
    diff_url: String,
    patch_url: String,
    issue_url: String,
    commits_url: String,
    review_comments_url: String,
    review_comment_url: String,
    statuses_url: String,
    number: String,
    state: String,
    locked: String,
}

/// The implementation for `GitHubOptions`
impl GitHub {
    /// Create a new GitHub struct.
    ///
    /// # Arguments
    ///
    /// * `github_token` - The Github Token
    /// * `github_url` - The Github API Url
    pub fn new(github_token: &str, github_url: &str) -> Self {
        let user_name =
            get_value_from_api(github_url, github_token, "login", "user").unwrap_or_default();
        let g = GitHub {
            github_token: github_token.to_string(),
            github_url: github_url.to_string(),
            github_username: user_name,
        };
        return g;
    }

    pub fn push(
        self,
        repo: &Repository,
        to_branch: String,
        from_branch: String,
        message: String,
    ) -> Result<PullResponse, Box<dyn std::error::Error>> {
        debug!("Pushing commits from {} to {}", from_branch, to_branch);
        let binding = PathBuf::from(repo.path());
        let path_str = binding.to_str().expect("Unable to get repo name");
        let parts = path_str.split(MAIN_SEPARATOR);
        let url = format!(
            "{}/repos/{}/{}/pulls",
            self.github_url,
            self.github_username,
            parts.last().expect("Cannot get Repo Name")
        );
        debug!("Posting to {}", url);
        let client = self.get_client();
        // set the body
        let mut map = HashMap::new();
        map.insert("title", "AI Generated Pull Request");
        map.insert("head", &from_branch);
        map.insert("base", &to_branch);
        map.insert("body", &message);
        info!("Sending push request to {}", url);
        let res = client.post(url).json(&map).send()?;
        let data = res.json::<PullResponse>()?;
        return Ok(data);
    }
    fn get_client(self) -> reqwest::blocking::Client {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/vnd.github+json".parse().unwrap());
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.github_token).parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        let client = reqwest::blocking::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Error Building Reqwest Client");
        return client;
    }
}

/// The implementation of `Git`
impl<'a> Git<'a> {
    /// Create a new Git struct.  Everything but the path is optional
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the repo, should almost always be `.`
    /// * `auto_add` -  Should all untracked changes be added to the index before the commit
    /// * `auto_push` - Should git push be called on the branch before the pr
    /// * `sign_commits` - Should commits be pgp signed
    /// * `key_id` - The key id, only matters if `sign_commits` is `true`
    /// * `user_name` - The git user name
    /// * `user_email` - The git user email
    pub fn new(
        path: &'a str,
        auto_add: Option<&'a bool>,
        auto_push: Option<&'a bool>,
        sign_commits: Option<&'a bool>,
        key_id: Option<&'a str>,
        user_name: Option<&'a str>,
        user_email: Option<&'a str>,
        ssh_key_path: Option<&'a str>,
        ssh_user_name: Option<&'a str>,
    ) -> Self {
        let g = Git {
            path,
            auto_add,
            auto_push,
            sign_commits,
            key_id,
            user_name,
            user_email,
            ssh_key_path,
            ssh_user_name,
        };
        return g;
    }

    /// Opens the repository
    pub fn open_repository(self) -> Result<Repository, git2::Error> {
        debug!("Getting repository");
        let repo = Repository::open(self.path)?;
        return Ok(repo);
    }

    /// find the last commit to this repo
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository
    pub fn find_last_commit(self, repo: &Repository) -> Result<Commit, git2::Error> {
        debug!("Finding last commit");
        let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
        obj.into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find last commit"))
    }

    /// Adds all untracked files to repo (same as running `git add .`)
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository
    fn add_all(self, repo: &Repository) -> Result<(), git2::Error> {
        debug!("Adding all files to the index");
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        return index.write();
    }

    /// Gets the diff on what is going to be committed.  If `auto_add` is false
    /// only files you added to the index yourself will be committed.
    ///
    /// If you want to see what will be sent this is the equivalent of `git diff --cached`
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository
    pub fn get_commit_diff(self, repo: &Repository) -> Result<Diff, git2::Error> {
        debug!("Creating commit");
        let last_commit = self.find_last_commit(repo)?;
        // some helpful debug stuff
        if log_enabled!(Level::Debug) {
            debug!("Last commit:");
            debug!("{}", self.display_commit(&last_commit));
        }
        // check for auto add
        if *self.auto_add.unwrap_or(&false) {
            debug!("Automatically adding all files to index");
            self.add_all(repo)?;
        }
        // ready to diff
        let index = repo.index()?;
        let old_tree = last_commit.tree()?;
        debug!("Index and Old Tree Prepared, Ready to Diff");
        let diff = repo.diff_tree_to_index(
            Some(&old_tree),
            Some(&index),
            Some(&mut DiffOptions::default()),
        )?;
        return Ok(diff);
    }

    /// Convient method to turn a `Diff` to a `String`
    /// Will panic if there are any non-UTF8 characters in the generated diff
    /// although I don't know how that could happen
    ///
    /// # Arguments
    ///
    /// * `diff` - The diff
    pub fn diff_to_string(&self, diff: &Diff) -> Result<String, git2::Error> {
        debug!("Turning diff to a string");
        let mut diff_content = String::new();
        diff.print(
            DiffFormat::Patch,
            |_delta: DiffDelta, _hunk: Option<DiffHunk>, line: DiffLine| {
                let line_num = match line.old_lineno() {
                    Some(num) => num,
                    None => 0,
                };

                let a_line =
                    std::str::from_utf8(&line.content()).expect("Non UTF8 Characters in Diff");

                if a_line.starts_with("diff --git") || a_line.starts_with("@@") {
                    diff_content.push_str(&format!(
                        "{}",
                        std::str::from_utf8(&line.content()).expect("Non UTF8 Characters in Diff")
                    ));
                } else {
                    match line.origin() {
                        '-' => diff_content.push_str("-"),

                        '+' => diff_content.push_str("+"),

                        _ => diff_content.push_str(" "),
                    };
                    diff_content.push_str(&format!("{}", line_num));
                    diff_content.push_str(&format!(
                        " {}",
                        std::str::from_utf8(&line.content()).expect("Non UTF8 Characters in Diff")
                    ));
                }

                true
            },
        )?;
        return Ok(diff_content);
    }

    /// Convient method to pretty-print a commit
    ///
    /// # Arguments
    ///
    /// * `commit` - The commit
    pub fn display_commit(&self, commit: &Commit) -> String {
        let timestamp = commit.time().seconds();
        let tm = time::at(time::Timespec::new(timestamp, 0));
        let res = format!(
            "commit {}\nAuthor: {}\nDate:   {}\n\n    {}",
            commit.id(),
            commit.author(),
            tm.rfc822(),
            commit.message().unwrap_or("no commit message")
        );
        return res;
    }

    /// Actually make the commit
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository
    /// * `msg` - The commit message: hopefully from the AI
    pub fn make_commit(&self, repo: &Repository, msg: &str) -> Result<Oid, git2::Error> {
        debug!("Performing commit");
        let git_config = repo.config()?;
        let user_name = match self.user_name {
            Some(name) => name,
            None => git_config.get_str("user.name")?,
        };
        let user_email = match self.user_email {
            Some(email) => email,
            None => git_config.get_str("user.email")?,
        };
        debug!("{} {} is doing the commit", &user_name, &user_email);
        let sig = Signature::now(user_name, user_email)?;
        let last_commit = self.find_last_commit(repo)?;
        let index_tree_id = repo.index()?.write_tree()?;
        let index_tree = repo.find_tree(index_tree_id)?;
        let commit_id = repo.commit(Some("HEAD"), &sig, &sig, msg, &index_tree, &[&last_commit])?;
        if log_enabled!(Level::Debug) {
            debug!("New commit:");
            debug!("{}", self.display_commit(&repo.find_commit(commit_id)?));
        }
        return Ok(commit_id);
    }
    /// Push the branch to remote
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository
    /// * `branch_name` - The branch name, should be the current one
    pub fn push_to_remote(&self, repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
        debug!("Pushing branch to origin for PR");
        let mut remote = repo.find_remote("origin")?;
        debug!("Found origin, creating ssh callback");
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_, username_from_url, _| {
            Cred::ssh_key_from_agent(username_from_url.unwrap())
        });
        debug!("Callback created, time to push");
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);
        debug!("Getting Branch to Push");
        let branch = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .unwrap();
        let refname = format!(
            "refs/heads/{}",
            branch
                .name()
                .unwrap()
                .expect("Unable to unwrape the branch name")
                .trim_start_matches("refs/heads/")
        );
        return remote.push(&[&refname], Some(&mut push_opts));
    }
}

// Helper functions
fn get_value_from_api(
    base_url: &str,
    token: &str,
    key: &str,
    url_tail: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/{}", base_url, url_tail);
    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token)).expect("Unable to set Auth Header"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );

    let response = client
        .get(&url)
        .headers(headers)
        .send()?
        .json::<serde_json::Value>()?;

    if let Some(value) = response.get(key) {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    Err("Unable to extract value from API response".into())
}
