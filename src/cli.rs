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

    /// Compute object ID and optionally creates a blob from a file
    HashObject(HashObjectArgs),
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<String>,
}

#[derive(Args)]
pub struct CatFileArgs {
    /// Specify the type
    pub mode: CatFileMode,

    /// The object to display
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

#[derive(Args)]
pub struct HashObjectArgs {
    /// Specify the type
    #[arg(short, long)]
    pub types: HashObjectType,

    /// Actually write the object into the .git directory
    #[arg(short, long)]
    pub write: bool,

    /// Read object from <file>
    pub path: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "lower")]
pub enum HashObjectType {
    Blob,
    Commit,
    Tag,
    Tree,
}
