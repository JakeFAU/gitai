use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use yaml_rust::{Yaml, YamlLoader};

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
}

fn load_yaml_file(file: &str) -> Yaml {
    let f = Path::new(file);
    let display = f.display();
    let mut file = match File::open(&f) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Unable to read file");

    let mut docs = YamlLoader::load_from_str(&contents).unwrap();
    let doc = docs.swap_remove(0);

    return doc;
}

fn main() {
    let cli = Cli::parse();

    let config_file: PathBuf = match cli.config {
        None => {
            let home_dir = dirs_next::home_dir().expect("User Home Dir Not Set");
            let mut path = PathBuf::new();
            path.push(home_dir.as_os_str());
            path.push(".gitai");
            path.push("settings.yaml");
            path
        }
        Some(c) => c,
    };

    let settings = load_yaml_file(&config_file.into_os_string().into_string().unwrap());

    let git_remote_token: String = match cli.git_remote_token {
        Some(token) => token,
        None => match env::var("GIT_TOKEN") {
            Ok(token) => token,
            Err(..) => {
                if settings["GIT_TOKEN"].is_badvalue() {
                    String::new()
                } else {
                    String::from(settings["GIT_TOKEN"].as_str().unwrap())
                }
            }
        },
    };

    let git_remote_url: String = match cli.git_remote_url {
        Some(url) => url,
        None => match env::var("GIT_URL") {
            Ok(url) => url,
            Err(..) => {
                if settings["GIT_URL"].is_badvalue() {
                    String::new()
                } else {
                    String::from(settings["GIT_URL"].as_str().unwrap())
                }
            }
        },
    };

    let ai_token: String = match cli.open_ai_token {
        Some(token) => token,
        None => match env::var("AI_TOKEN") {
            Ok(token) => token,
            Err(..) => {
                if settings["AI_TOKEN"].is_badvalue() {
                    panic!("The AI_TOKEN must be set somewhere")
                } else {
                    String::from(settings["AI_TOKEN"].as_str().unwrap())
                }
            }
        },
    };

    let ai_url: String = match cli.open_ai_url {
        Some(url) => url,
        None => match env::var("AI_URL") {
            Ok(url) => url,
            Err(..) => {
                if settings["AI_URL"].is_badvalue() {
                    panic!("The AI_URL must be set somewhere")
                } else {
                    String::from(settings["AI_URL"].as_str().unwrap())
                }
            }
        },
    };

    let local_repo: PathBuf = match cli.local_repo {
        Some(repo) => repo,
        None => PathBuf::from("."),
    };

    match &cli.command {
        Some(Commands::Commit {}) => {
            println!("commit");
            println!("{:?}", local_repo);
            println!("{ai_token}");
            println!("{ai_url}");
        }
        Some(Commands::PR { from, to }) => {
            println!("pr {from} -> {to}");
            println!("{:?}", local_repo);
            println!("{ai_token}");
            println!("{ai_url}");
            println!("{git_remote_token}");
            println!("{git_remote_url}");
        }
        None => {}
    }
}
