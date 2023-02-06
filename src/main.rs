use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fs::{self};

use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub mod ai;
pub mod git;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// set GitHub token
    #[arg(long = "git_token", value_name = "GIT_TOKEN")]
    git_remote_token: Option<String>,

    /// set Git remote url
    #[arg(long = "git_url", value_name = "GIT_URL")]
    git_remote_url: Option<String>,

    /// set OpenAI token
    #[arg(long = "ai_token", value_name = "AI_TOKEN")]
    open_ai_token: Option<String>,

    /// set OpenAI url
    #[arg(long = "ai_url", value_name = "AI_URL")]
    open_ai_url: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Sets a custom local repo, you should probably not use this
    #[arg(short, long, value_name = "REPO")]
    local_repo: Option<PathBuf>,

    /// Turn Verbose Mode on
    #[arg(short, long)]
    verbose: bool,

    /// Turn Stochastic Mode on
    #[arg(short, long)]
    stochastic: bool,

    /// Turns Auto Add mode on which adds . to git before making the commit DANGEROUS
    #[arg(short, long)]
    auto_add: bool,

    /// Turns Auto AI mode on automatically accepts the AI message without review DANGEROUS
    #[arg(short = 'i', long = "ai")]
    auto_ai: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
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
    /// Get AI Models
    Models {},
}

/// JSON Structs
#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    pub ai_information: AiInformation,
    pub git_information: Option<GitInformation>,
}
#[derive(Serialize, Deserialize, Debug)]
struct AiInformation {
    pub api_url: String,
    pub api_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitInformation {
    pub remote_token: Option<String>,
    pub remote_url: Option<String>,
}

fn load_json_file(path: PathBuf) -> Result<Settings, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let settings_file_as_string =
        fs::read_to_string(path.as_os_str()).expect("Unable to read file");
    let settings = serde_json::from_str(&settings_file_as_string).expect("JSON not parsing");
    Ok(settings)
}

fn main() {
    let cli = Cli::parse();

    let config_file: PathBuf = match cli.config {
        None => {
            let home_dir = dirs_next::home_dir().expect("User Home Dir Not Set");
            let mut path = PathBuf::new();
            path.push(home_dir.as_os_str());
            path.push(".gitai");
            path.push("settings.json");
            path
        }
        Some(c) => c,
    };

    let settings = load_json_file(config_file).expect("Problem reading settings.json");

    let open_ai_token: String = if let Some(a) = cli.open_ai_token {
        a
    } else {
        settings.ai_information.api_token
    };

    let open_ai_url: String = if let Some(a) = cli.open_ai_url {
        a
    } else {
        settings.ai_information.api_url
    };

    let git_remote_token: Option<String> = if let Some(g) = cli.git_remote_token {
        Some(g)
    } else {
        if let Some(g) = &settings.git_information {
            g.remote_token.to_owned()
        } else {
            None
        }
    };

    let git_remote_url: Option<String> = if let Some(i) = cli.git_remote_url {
        Some(i)
    } else {
        if let Some(g) = &settings.git_information {
            g.remote_url.to_owned()
        } else {
            None
        }
    };

    let git_local_path = if let Some(i) = cli.local_repo {
        i
    } else {
        PathBuf::from(".")
    };

    // Flags
    let verbose: bool = cli.verbose;
    let stochastic: bool = cli.stochastic;
    let auto_add: bool = cli.auto_add;
    let auto_ai: bool = cli.auto_ai;

    match &cli.command {
        Some(Commands::Commit {}) => {
            let git = git::Git::new_local_only(git_local_path, Some(auto_add), Some(auto_ai));
            let repo = git.open_repo().unwrap();
            let origin = repo.find_remote("origin").unwrap();
        }
        Some(Commands::PR { from, to }) => {
            println!("pr {from} -> {to}");
        }
        Some(Commands::Models {}) => {
            let ai = ai::AI::new(&open_ai_url, &open_ai_token);
            let models = ai.get_models();
            print!("{:#?}", models);
        }
        None => {}
    }
}
