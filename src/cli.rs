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

    /// Check the status of the repository
    Status {},

    /// Provide content of repository objects
    CatFile(CatFileArgs),

    /// Compute object ID and optionally creates a blob from a file
    HashObject(HashObjectArgs),

    /// Display history of a given commit
    Log(LogArgs),

    /// Pretty-print a tree object.
    LsTree(LsTreeArgs),

    /// Checkout a commit inside of a directory
    Checkout(CheckoutArgs),

    /// List references
    ShowRef {},

    /// List and create tags
    Tag(TagArgs),

    /// Parse revision (or other objects) identifiers
    RevParse(RevParseArgs),

    /// Show information about files in the index and the working tree
    LsFiles(LsFilesArgs),

    /// Add file contents to the index
    Add(AddArgs),

    /// Remove files from the working tree and from the index
    Rm(RmArgs),

    /// Record changes to the repository
    Commit(CommitArgs),

    /// Create a tree object from the current index
    WriteTree,
}

#[derive(Args)]
pub struct InitArgs {
    pub path: Option<String>,
}

#[derive(Args)]
pub struct CatFileArgs {
    /// Specify the type
    pub mode: ObjectMode,

    /// The object to display
    pub object: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "lower")]
pub enum ObjectMode {
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

#[derive(Args)]
pub struct LogArgs {
    /// Commit to start at
    pub commit_hash: Option<String>,
}

#[derive(Args)]
pub struct LsTreeArgs {
    /// Recurse into sub-trees
    #[arg(short)]
    pub recursive: bool,

    /// A tree-ish object
    pub tree: String,
}

#[derive(Args)]
pub struct CheckoutArgs {
    /// The commit or tree to checkout
    pub commit: String,

    /// The EMPTY directory to checkout on
    pub path: String,
}

#[derive(Args)]
pub struct TagArgs {
    /// Whether to create a tag object
    #[arg(short)]
    pub add: bool,

    /// The new tag's name
    pub name: Option<String>,

    /// The object the new tag will point to
    #[arg(default_value_t = String::from("HEAD"))]
    pub object: String,
}

#[derive(Args)]
pub struct RevParseArgs {
    /// Specify the expected type
    #[arg(long = "type")]
    pub object_type: Option<ObjectMode>,

    /// The name to parse
    pub name: String,
}

#[derive(Args)]
pub struct LsFilesArgs {
    /// Show staged contents' mode bits, object name and stage
    #[arg(short)]
    pub stage: bool,
}

#[derive(Args)]
pub struct AddArgs {
    /// Files to add content from
    #[arg(required = true)]
    pub paths: Vec<String>,
}

#[derive(Args)]
pub struct RmArgs {
    /// Only remove from the index, leave files in the working tree
    #[arg(long)]
    pub cached: bool,

    /// Override the safety check for unstaged modifications
    #[arg(short)]
    pub force: bool,

    /// Files to remove
    #[arg(required = true)]
    pub paths: Vec<String>,
}

#[derive(Args)]
pub struct CommitArgs {
    /// Commit message
    #[arg(short)]
    pub message: String,
}
