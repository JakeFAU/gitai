use clap::{Parser, Subcommand};
use dirs_next::home_dir;
use log::{debug, error, info, log_enabled, Level};
use serde_json::{from_reader, Value};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::{env, fs};
use termion::input::TermRead;
use termios::{tcsetattr, Termios, TCSAFLUSH};

use crate::ai::{OpenAiClient, Prompt};
use crate::git::{Git, GitHubOptions};

pub mod ai;
pub mod git;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// set GitHub API token
    #[arg(long = "git_api_token", value_name = "GITHUB_TOKEN")]
    github_token: Option<String>,

    /// set GitHub API url
    #[arg(long = "git_api_url", value_name = "GITHUB_URL", value_hint = clap::ValueHint::Url)]
    github_url: Option<String>,

    /// set OpenAI token
    #[arg(long = "ai_api_token", value_name = "AI_TOKEN")]
    open_ai_token: Option<String>,

    /// set OpenAI url
    #[arg(long = "ai_api_url", value_name = "AI_URL", value_hint = clap::ValueHint::Url)]
    open_ai_url: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE", value_hint = clap::ValueHint::DirPath)]
    config: Option<PathBuf>,

    /// Sets a custom local repo, you should probably not use this
    #[arg(short, long, value_name = "REPO", value_hint = clap::ValueHint::DirPath)]
    local_repo: Option<PathBuf>,

    /// Turn Verbose Mode on
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    verbose: Option<bool>,

    /// Turn Stochastic Mode on
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    stochastic: Option<bool>,

    /// Turns Auto Add mode on which adds . to git before making the commit DANGEROUS
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    auto_add: Option<bool>,

    /// Turns Auto AI mode on automatically accepts the AI message without review DANGEROUS
    #[arg(short = 'i', long, action = clap::ArgAction::SetTrue)]
    auto_ai: Option<bool>,

    /// Turns Auto Push mode on which pushes local to remote before the pr, detfaults to true
    #[arg(short = 'u', long, action = clap::ArgAction::SetFalse)]
    auto_push: Option<bool>,

    /// Number of times to try the AI: Note OpenAI Chatbot is not Idenpotent
    #[arg(short, long, value_name = "TRIES", value_parser=_allowed_num_tries)]
    num_tries: Option<u64>,

    /// Sign Commits, if set some variables must be added to settings.json
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    gpg_sign_commit: Option<bool>,

    /// Programming Language, very useful for small commits/pr
    #[arg(short, long, value_name = "LANGUAGE")]
    programming_language: Option<String>,

    /// Signing Key ID: Note, ignored if sign_commit=false
    #[arg(long)]
    signature_id: Option<String>,

    /// The path to the ssh key
    #[arg[long]]
    ssh_key_path: Option<String>,

    /// The ssh user, i personally have never seen this anything but `git`
    #[arg[long]]
    ssh_user: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate Commit Message
    Commit {},
    /// Generare Pull Request
    PR {
        /// The from branch
        from: String,
        /// The to branch
        to: String,
    },
    /// Get AI Models - Good for testing connectivity
    Models {},
}

fn get_settings(p: Option<PathBuf>) -> Result<Value, Box<dyn std::error::Error>> {
    let default_path: PathBuf = [
        home_dir()
            .expect("HOMEDIR Not Set")
            .to_str()
            .expect("Invalid HOMEDIR"),
        ".gitai",
        "settings.json",
    ]
    .iter()
    .collect();
    let settings_path = p.unwrap_or(default_path);
    debug!("Checking {:#?} to see if it exists", &settings_path);
    if settings_path.exists() {
        let mut file = File::open(settings_path.as_path()).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("couldn't read the file");
        let data: serde_json::Value =
            from_reader(contents.as_bytes()).expect("couldn't parse the JSON");
        return Ok(data);
    } else {
        debug!("{:#?} does not exist, creating a blank one", &settings_path);
        fs::create_dir_all(
            &settings_path
                .parent()
                .expect("Cannot create the directory $HOME/.gitai"),
        )?;
        let mut f = File::create(&settings_path).unwrap();
        f.write_all(DEFAULT_FILE.as_bytes())
            .expect("Unable to write default file");
        return Err("No settings.json exists, we created a blank one at ~/.gitai".into());
    }
}

fn _allowed_num_tries(s: &str) -> Result<u8, String> {
    clap_num::number_range(s, 1, 3)
}

fn restore_terminal() -> io::Result<()> {
    let mut old_termios = Termios::from_fd(0)?;
    tcsetattr(0, TCSAFLUSH, &old_termios)?;
    Ok(())
}

/// Helper function to ask the user whether or not they really wanted to ____
/// (as specified by the `prompt`). As long as the response starts with the
/// letter `y` (case insensitive), the reply is treated as affirmative.
pub fn prompt_yes_no<S>(prompt: S) -> io::Result<bool>
where
    S: AsRef<str>,
{
    let prompt = prompt.as_ref();

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    write!(stdout, "{} [y/N] ", prompt)?;
    stdout.flush()?;

    match TermRead::read_line(&mut stdin)? {
        Some(ref reply) if reply.to_ascii_lowercase().starts_with('y') => Ok(true),
        _ => Ok(false),
    }
}

fn remove_blank_lines(input: &String) -> String {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

fn main() {
    env_logger::init();
    info!("Initializing GitAI");

    debug!("Parsing CLI");
    let cli = Cli::parse();

    debug!("Reading settings file");
    let settings = match get_settings(cli.config) {
        Ok(s) => s,
        Err(e) => panic!("No settings.json exists, we created a blank one at ~/.gitai"),
    };

    debug!("Setting Variables");
    let ai_token = cli
        .open_ai_token
        .or(env::var("AI_OPENAI_TOKEN").ok())
        .or(settings["ai_information"]["api_token"]
            .as_str()
            .map(|s| s.to_owned()))
        .expect("AI_TOKEN Must be set");

    let ai_url = cli
        .open_ai_url
        .or(env::var("AI_OPENAI_URL").ok())
        .or(settings["ai_information"]["api_url"]
            .as_str()
            .map(|s| s.to_owned()))
        .expect("AI_URL Must be set");

    let git_token = cli
        .github_token
        .or(env::var("AI_GIT_TOKEN").ok())
        .or(settings["git_information"]["api_token"]
            .as_str()
            .map(|s| s.to_owned()));

    let git_url = cli
        .github_url
        .or(env::var("AI_GIT_URL").ok())
        .or(settings["git_information"]["api_url"]
            .as_str()
            .map(|s| s.to_owned()));

    let language = cli
        .programming_language
        .or(settings["ai_information"]["options"]["language"]
            .as_str()
            .map(|s| s.to_owned()))
        .unwrap_or_default();

    // Flags
    let auto_ai = cli
        .auto_ai
        .or(settings["ai_information"]["options"]["auto_ai"].as_bool())
        .unwrap_or(false);

    let auto_add = cli
        .auto_add
        .or(settings["git_information"]["options"]["auto_add"].as_bool())
        .unwrap_or(false);

    let auto_push = cli
        .auto_push
        .or(settings["git_information"]["options"]["auto_push"].as_bool())
        .unwrap_or(true);

    let stochastic = cli
        .stochastic
        .or(settings["ai_information"]["options"]["stochastic"].as_bool())
        .unwrap_or(false);

    let num_tries = cli
        .num_tries
        .or(settings["ai_information"]["options"]["num_tries"].as_u64())
        .unwrap_or(1);

    let gpg_sign_commit = cli
        .gpg_sign_commit
        .or(settings["git_information"]["options"]["sign_commit"].as_bool())
        .unwrap_or(false);

    let gpg_key_id = cli.signature_id.or(Some(
        settings["git_information"]["options"]["gpg_key_id"].to_string(),
    ));

    let ssh_key_path = cli
        .ssh_key_path
        .as_deref()
        .or(settings["git_information"]["options"]["ssh_key_path"].as_str())
        .or(Some(&".ssh/id_rsa"));

    let ssh_user = cli
        .ssh_user
        .as_deref()
        .or(settings["git_information"]["options"]["ssh_user"].as_str())
        .or(Some(&"git"));

    let local_repo = cli.local_repo.unwrap_or(PathBuf::from("."));

    debug!("Variables Set OpenAI Url={:#?} should not be null", ai_url);
    debug!(
        "Local Repo={:#?} this should probably be '.' unless you have good reason",
        local_repo
    );

    debug!("Matching CLI Command");
    match &cli.command {
        Some(Commands::Commit {}) => {
            let git = Git::new(
                local_repo.to_str().unwrap_or("."),
                Some(&auto_add),
                Some(&auto_push),
                Some(&gpg_sign_commit),
                gpg_key_id.as_ref().map(|s| s.as_str()),
                None,
                None,
                ssh_key_path,
                ssh_user,
            );
            debug!("Getting Repository at {:#?}", &local_repo);
            let repo = git.open_repository().expect("Unable to open repository");

            debug!("Getting Diff for {:#?}", &local_repo);
            let diff = git.get_commit_diff(&repo).expect(
                "Unable to create git diff, try running git diff --cached to see if it works",
            );
            let git_diff_text = git
                .diff_to_string(&diff)
                .expect("Unable to parse generated git diff");

            debug!("Got Diff, Its OpenA Time");
            let client = OpenAiClient::new(ai_url, ai_token);

            debug!("We have a client, lets build the prompt");
            let mut prompt = Prompt::default();
            prompt.language = Some(language);
            prompt.git_diff = Some(git_diff_text);
            if log_enabled!(Level::Debug) {
                debug!("{}", prompt.git_diff.as_ref().unwrap());
            }
            let res = client
                .get_completions(prompt, None)
                .expect("Unable to get completions");

            let open_ai_choices = &res.choices.expect("OpenAI didn't send back any choices");
            let open_ai_first_completion = open_ai_choices
                .first()
                .expect("OpenAI didn't send back any choices");
            let mut open_ai_completion_text = open_ai_first_completion
                .text
                .as_ref()
                .expect("OpenAI didn't send back any message");

            let text = &remove_blank_lines(&open_ai_completion_text);

            println!("Here is your AI Generated Commit Message\n\n{}\n\n", text);

            let answer = prompt_yes_no("Would you like to use it?").expect("Error getting input");
            restore_terminal().expect("Unable to switch the terminal back to stout");
            debug!("Are we going to use this message? {}", answer);

            if answer {
                let oid = git.make_commit(&repo, &text).expect("Commit Failed");
                if log_enabled!(Level::Debug) {
                    debug!("Commit worked, returned {}", oid.to_string());
                    let new_commit = repo
                        .find_commit(oid)
                        .expect("Cammpt find new commit, thats odd.");
                    debug!("{}", git.display_commit(&new_commit))
                }

                info!("Commit Successful, OID={}", oid.to_string());
            } else {
                info!("Sorry, feel free to try again. OpenAi is not idenpotent");
                info!(
                    "You wasted {} tokens",
                    res.usage
                        .unwrap()
                        .total_tokens
                        .expect("OpenAI Didn't event send back how many tokens you used.")
                );
            }
        }
        Some(Commands::PR { from, to }) => {
            info!("Generating PR from {:#?} to {:#?}", from, to);
        }
        Some(Commands::Models {}) => {
            info!("Getting Available Models");
            let client = OpenAiClient::new(ai_url, ai_token);
            let res = client.get_models().expect("Unable to get models");
            print!("{:#?}", res)
        }
        None => (),
    }
}

const DEFAULT_FILE: &str = "
{
    \"ai_information\": {
      \"api_url\": \"<OPEN_API_URL>\",
      \"api_token\": \"<OPEN_AI_TOKEN>\",
      \"options\": {
        \"stochastic\": true
      }
    },
    \"git_information\": {
       \"remote_url\": \"<GITHUB_URL\",
      \"remote_token\": \"<GITHUB_TOKEN>\",
      \"options\": {
        \"sign_commits\": true
      }
    }
  }
";
