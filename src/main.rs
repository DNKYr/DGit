mod cli;
mod commands;
mod index;
mod object;
mod refs;
mod repository;

use crate::commands::{
    cat_file, checkout, hash_object, init, log, ls_tree, rev_parse, show_ref, tag,
};
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
                let msg = init::init(&std::path::PathBuf::from(p))?;
                println!("{msg}");
            } else {
                let msg = init::init(&current_directory_path)?;
                println!("{msg}");
            }
        }
        Commands::Status {} => match repository::repo_find(Some(&current_directory_path)) {
            Ok(repo) => {
                println!("{:?}", repo.get_git_dir().display());
            }

            Err(err_msg) => {
                println!("{err_msg}");
                process::exit(1);
            }
        },
        Commands::CatFile(args) => {
            cat_file::cmd_cat_file(args)?;
        }

        Commands::HashObject(args) => {
            hash_object::cmd_hash_object(args)?;
        }

        Commands::Log(args) => {
            log::cmd_log(args)?;
        }

        Commands::LsTree(args) => {
            ls_tree::cmd_ls_tree(args)?;
        }

        Commands::Checkout(args) => {
            checkout::cmd_checkout(args)?;
        }

        Commands::ShowRef {} => {
            show_ref::cmd_show_ref()?;
        }

        Commands::Tag(args) => {
            tag::cmd_tag(args)?;
        }

        Commands::RevParse(args) => {
            rev_parse::cmd_rev_parse(args)?;
        }
    }

    Ok(())
}
