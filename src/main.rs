use ai::OpenAiRequestParams;
use clap::{Parser, Subcommand};
use log::{debug, error, info};
use rand::seq::SliceRandom;

use std::io::{self, Write};
use std::path::PathBuf;
use termion::input::TermRead;
use termios::{tcsetattr, Termios, TCSAFLUSH};

use crate::ai::OpenAiClient;
use crate::git::{Git, GitHub};
use crate::settings::{AiPrompt, Settings};

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
    clap_num::number_range(s, 1, 5)
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

fn error_message(message: &str) -> String {
    error!("{}", message);
    return message.to_string();
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

            debug!("Got Diff, Its OpenAI Time");
            let client = OpenAiClient::new(ai_url, ai_token);

            debug!("We have a client, lets build the prompt");
            let mut completions: Vec<String> = Vec::new();
            if stochastic {
                info!("Stochastic Mode Set");
                let prompts = Settings::get_commit_prompt_choices();
                for i in 0..num_tries {
                    let mut prompt: AiPrompt =
                        prompts.choose(&mut rand::thread_rng()).unwrap().to_owned();
                    prompt.language = language.to_string();
                    prompt.git_diff = git_diff_text.to_string();
                    let params = OpenAiRequestParams {
                        prompt: format!("{}", prompt),
                        ..Default::default()
                    };
                    debug!("Post #{} to OpenAI", (i + 1));
                    let res = &client
                        .get_completions(prompt.to_owned(), params)
                        .expect("Cannot connect to API");
                    let open_ai_choices = res.choices.as_ref().unwrap();
                    let open_ai_first_completion = open_ai_choices.first().unwrap();
                    let open_ai_completion_text = open_ai_first_completion.text.as_ref().unwrap();
                    let text = remove_blank_lines(&open_ai_completion_text);
                    completions.push(text);
                }
            } else {
                info!("Non-Stochastic Mode Set");
                let mut prompt = AiPrompt::default();
                prompt.language = language;
                prompt.git_diff = git_diff_text;
                let params = OpenAiRequestParams {
                    prompt: format!("{}", prompt),
                    n: Some(num_tries),
                    ..Default::default()
                };
                debug!("Posting to OpenAI");
                let res = client
                    .get_completions(prompt, params)
                    .expect("Cannot connect to API");
                let open_ai_choices = res.choices.unwrap();
                for choice in open_ai_choices {
                    let text = remove_blank_lines(
                        &choice
                            .text
                            .expect("OpenAI Responded but with no completions"),
                    );
                    completions.push(text);
                }
            }

            println!("Here is your AI Generated Commit Message\n\n");
            for comp in completions.iter() {
                println!("{}", comp)
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
