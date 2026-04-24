use clap::{Args, Parser, Subcommand};
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// initialize .git directory
    Init(InitArgs),

    /// Check if the current path is within a git directory
    Status {},
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<String>,
}
