mod cli;
mod object;
mod repo;

use clap::Parser;
use cli::{Cli, Commands};
use std::env;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let current_directory_path = env::current_dir()?;

    match &cli.command {
        Commands::Init(args) => {
            if let Some(p) = &args.path {
                let msg = repo::repo_create(&std::path::PathBuf::from(p))?;
                println!("{msg}");
            } else {
                let msg = repo::repo_create(&current_directory_path)?;
                println!("{msg}");
            }
        }
        Commands::Status {} => match repo::repo_find(Some(&current_directory_path)) {
            Ok(repo) => {
                println!("{:?}", repo.get_git_dir().display());
            }

            Err(err_msg) => {
                println!("{err_msg}");
                process::exit(1);
            }
        },
        Commands::CatFile(args) => {
            repo::cmd_cat_file(args)?;
        }

        Commands::HashObject(args) => {
            repo::cmd_hash_object(args)?;
        }
    }

    Ok(())
}
