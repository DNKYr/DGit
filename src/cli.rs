use clap::{Args, Parser, Subcommand, ValueEnum};
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

    /// Provide content of repository objects
    CatFile(CatFileArgs),
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<String>,
}

#[derive(Args)]
pub struct CatFileArgs {
    pub mode: CatFileMode,
    pub object: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "lower")]
pub enum CatFileMode {
    Blob,
    Tree,
    Commit,
    Tag,
}
