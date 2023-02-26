use clap::{Parser, Subcommand};
use log::{debug, info, log_enabled, Level};

use std::io::{self, Write};
use std::path::PathBuf;
use termion::input::TermRead;
use termios::{tcsetattr, Termios, TCSAFLUSH};

use crate::ai::{OpenAiClient, Prompt};
use crate::git::{Git, GitHub};
use crate::settings::Settings;

pub mod ai;
pub mod git;
pub mod settings;

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
    num_tries: Option<u8>,

    /// Sign Commits, if set some variables must be added to settings.json
    #[arg(long, action = clap::ArgAction::SetTrue)]
    gpg_sign_commit: Option<bool>,

    /// the signing key, only matters if `gpg_sign_commit` is true.
    #[arg(long)]
    gpg_key_id: Option<String>,

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

fn _allowed_num_tries(s: &str) -> Result<u8, String> {
    clap_num::number_range(s, 1, 3)
}

fn restore_terminal() -> io::Result<()> {
    let old_termios = Termios::from_fd(0)?;
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
    let settings = Settings::new().expect("Unable to load settings file at ~/.gitai/settings.json");

    debug!("Setting Variables");
    //ai variables
    let ai_token = cli.open_ai_token.unwrap_or(settings.ai_settings.api_key);
    let ai_url = cli.open_ai_url.unwrap_or(settings.ai_settings.api_url);
    debug!("AI Variables Set url={}", ai_url);

    // github variables
    let github_token = cli
        .github_token
        .unwrap_or(settings.git_settings.github_api_key);
    let github_url = cli
        .github_url
        .unwrap_or(settings.git_settings.github_api_url);
    debug!("GitHub Variables Set url={}", github_url);

    // other variables - not flags first
    let language = cli
        .programming_language
        .or(Some(settings.ai_settings.ai_options.prompt.language))
        .unwrap_or("Python".to_string());

    let num_tries = cli
        .num_tries
        .or(Some(settings.ai_settings.ai_options.n))
        .unwrap_or(1);

    let ssh_key_path = cli
        .ssh_key_path
        .or(Some(settings.git_settings.git_options.ssh_key_path))
        .unwrap_or("~/.ssh/id_rsa".to_string());

    let ssh_user =
        Some(settings.git_settings.git_options.ssh_user_name).unwrap_or("git".to_string());

    let local_repo = cli
        .local_repo
        .or(Some(settings.git_settings.git_options.local_path))
        .unwrap_or(PathBuf::from("."));

    let gpg_key_id = cli
        .gpg_key_id
        .or(Some(settings.git_settings.git_options.key_id))
        .unwrap_or_default();

    // Flags
    let auto_ai = cli
        .auto_ai
        .or(Some(settings.ai_settings.ai_options.auto_ai))
        .unwrap_or(false);

    let auto_add = cli
        .auto_add
        .or(Some(settings.git_settings.git_options.auto_add))
        .unwrap_or(false);

    let auto_push = cli
        .auto_push
        .or(Some(settings.git_settings.git_options.auto_push))
        .unwrap_or(true);

    let stochastic = cli
        .stochastic
        .or(Some(settings.ai_settings.ai_options.stochastic))
        .unwrap_or(false);

    let gpg_sign_commits = cli
        .gpg_sign_commit
        .or(Some(settings.git_settings.git_options.sign_commits))
        .unwrap_or(false);

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
                Some(&gpg_sign_commits),
                Some(&gpg_key_id),
                None,
                None,
                Some(&ssh_key_path),
                Some(&ssh_user),
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
            let open_ai_completion_text = open_ai_first_completion
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
            let g_hub = GitHub::new(github_token.as_str(), github_url.as_str());
            println!("{:#?}", g_hub)
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
