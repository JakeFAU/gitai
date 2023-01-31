use clap::{arg, Command};

fn cli() -> Command {
    Command::new("gitai")
        .about("Using AI to help GIT")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("commit")
                .about("Commits to a repo")
                .arg(arg!(<LOCATION> "The location of the local repo"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("pr")
                .about("Creates a PR")
                .arg(arg!(<FROM_BRANCH> "The from branch"))
                .arg(arg!(<TO_BRANCH> "The to branch"))
                .arg_required_else_help(true),
        )
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("commit", sub_matches)) => {
            println!(
                "Committing {}",
                sub_matches.get_one::<String>("LOCATION").expect("required")
            );
        }
        Some(("pr", sub_matches)) => {
            let from_branch = sub_matches
                .get_one::<String>("FROM_BRANCH")
                .expect("required");
            let to_branch = sub_matches
                .get_one::<String>("TO_BRANCH")
                .expect("required");
            println!("Creating Pull Request {from_branch} to {to_branch}");
        }
        _ => panic!("Unknown command"),
    }
}
