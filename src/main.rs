use clap::Parser;
use wax::app::run;
use wax::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli).await {
        eprintln!("{err}");
        std::process::exit(err.exit_code());
    }
}
