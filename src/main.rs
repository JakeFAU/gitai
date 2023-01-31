use clap::Parser;

/// Use AI to make Git easier
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Location of the local repo
    #[arg(short, long, default_value = ".")]
    location: String,
}

fn main() {
    let args = Args::parse();

    println!("Repo Path={}", args.location);
}
