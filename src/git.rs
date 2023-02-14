use git2::{
    Commit, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, IndexAddOption,
    ObjectType, Oid, Repository, Signature,
};
use log::{debug, log_enabled, Level};

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
        }
    }
}

/// GitGub Options
#[derive(Debug)]
pub struct GitHubOptions<'a> {
    /// The GitHub API Token
    github_token: &'a str,
    /// The GitHub API URL
    github_url: &'a str,
}

/// The implementation for `GitHubOptions`
impl<'a> GitHubOptions<'a> {
    /// Create a new GitHubOptions struct.  Notice nohing is optional
    ///
    /// # Arguments
    ///
    /// * `github_token` - The Github Token
    /// * `github_url` - The Github API Url
    pub fn new(github_token: &'a str, github_url: &'a str) -> Self {
        GitHubOptions {
            github_token,
            github_url,
        }
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
    ) -> Self {
        let g = Git {
            path,
            auto_add,
            auto_push,
            sign_commits,
            key_id,
            user_name,
            user_email,
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
        let p = diff.print(
            DiffFormat::Patch,
            |delta: DiffDelta, hunk: Option<DiffHunk>, line: DiffLine| {
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
}
