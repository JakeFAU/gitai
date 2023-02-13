use git2::{
    Commit, Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine, DiffOptions, IndexAddOption,
    ObjectType, Oid, Repository, Signature,
};
use log::{debug, log_enabled, Level};

use std::path::PathBuf;

#[derive(Debug)]
pub struct GitOptions {
    path: Option<PathBuf>,
    git_token: Option<String>,
    git_url: Option<String>,
    auto_add: Option<bool>,
}

impl Default for GitOptions {
    fn default() -> Self {
        GitOptions {
            path: Some(PathBuf::from(".")),
            git_token: None,
            git_url: None,
            auto_add: Some(false),
        }
    }
}

impl GitOptions {
    pub fn new() -> Self {
        debug!("Getting default Options");
        GitOptions::default()
    }

    pub fn new_with_remote(git_token: &str, git_url: &str) -> Self {
        debug!("Getting Options with the Remote API Info");
        let go = GitOptions {
            git_token: Some(git_token.to_string()),
            git_url: Some(git_url.to_string()),
            path: Some(PathBuf::from(".")),
            auto_add: Some(false),
        };
        return go;
    }

    pub fn new_full(path: &PathBuf, git_token: &str, git_url: &str, auto_add: &bool) -> Self {
        debug!("Getting options with everything set");
        let go = GitOptions {
            git_token: Some(git_token.to_string()),
            git_url: Some(git_url.to_string()),
            path: Some(path.to_path_buf()),
            auto_add: Some(*auto_add),
        };
        return go;
    }
}
pub fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn display_commit(commit: &Commit) -> String {
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

fn _add_all(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    return index.write();
}

pub fn get_repository(git_options: &GitOptions) -> Repository {
    debug!("Getting repository");
    let path = git_options
        .path
        .as_deref()
        .expect("Cannot Create the Path Object to the repo");
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };
    debug!(
        "Repo Path={:#?} state={:#?}",
        repo.path().display(),
        repo.state()
    );
    return repo;
}

pub fn get_commit_diff<'a>(repo: &'a Repository, git_options: &'a GitOptions) -> Diff<'a> {
    debug!("Getting Diff between index and HEAD");
    let last_commit = find_last_commit(repo).expect("Cannot get last commit");
    if log_enabled!(Level::Debug) {
        debug!("{}", display_commit(&last_commit));
    }
    if git_options.auto_add.unwrap_or(false) {
        debug!("Add flag set, adding all files to index before diff");
        _add_all(repo).expect("Error Adding Files to Index");
    }
    let index = repo.index().expect("Cannot get repo index");
    let old = last_commit.tree().expect("Unable to get most recent tree");
    let diff = repo
        .diff_tree_to_index(Some(&old), Some(&index), Some(&mut DiffOptions::default()))
        .expect("Cannot generate DIFF");
    return diff;
}

pub fn get_diff_text<'a>(diff: &'a Diff, git_options: &'a GitOptions) -> String {
    let mut diff_content = String::new();
    let p = diff.print(
        DiffFormat::Patch,
        |delta: DiffDelta, hunk: Option<DiffHunk>, line: DiffLine| {
            let line_num = match line.old_lineno() {
                Some(num) => num,
                None => 0,
            };

            let a_line = std::str::from_utf8(&line.content()).expect("Non UTF8 Characters in Diff");

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
    );
    match p {
        Ok(..) => debug!("We did it, we printed the diff"),
        Err(..) => debug!("I guess not"),
    }
    return diff_content;
}

pub fn make_commit(
    repo: &Repository,
    message: &str,
    settings: &serde_json::Value,
) -> Result<Oid, git2::Error> {
    debug!("Committing files to repo");

    let git_config = repo.config()?;
    let user_email = match settings["git_information"]["options"]["user_email"].as_str() {
        Some(email) => email.to_string(),
        None => git_config.get_string("user.name")?.to_string(),
    };
    let user_name = match settings["git_information"]["options"]["user_name"].as_str() {
        Some(email) => email.to_string(),
        None => git_config.get_string("user.name")?.to_string(),
    };
    debug!(
        "Using the following values for the commit {} {}",
        user_name, user_email
    );
    let sig = Signature::now(&user_name, &user_email).expect("Error Generating Signature");

    debug!("Preparing actual commit");
    let last_commit = find_last_commit(repo).expect("Cannot get last commit");
    let tree_id = repo.index()?.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let commit_id = repo.commit(
        Some("HEAD"),    //  point HEAD to our new commit
        &sig,            // author
        &sig,            // committer
        message,         // commit message
        &tree,           // tree
        &[&last_commit], // parents
    )?;
    debug!("Commit worked id = {}", commit_id.to_string());
    return Ok(commit_id);
}
