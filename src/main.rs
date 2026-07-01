mod cli;
mod commands;
mod config;
mod gitignore;
mod index;
mod object;
mod refs;
mod repository;

use crate::commands::{
    add, cat_file, checkout, commit, hash_object, init, log, ls_files, ls_tree, rev_parse, rm,
    show_ref, status, tag, write_tree,
};
use clap::Parser;
use cli::{Cli, Commands};
use std::env;

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
        Commands::Status {} => {
            status::cmd_status()?;
        }
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
        Commands::LsFiles(args) => {
            ls_files::cmd_ls_files(args)?;
        }
        Commands::Add(args) => {
            add::cmd_add(args)?;
        }
        Commands::Rm(args) => {
            rm::cmd_rm(args)?;
        }
        Commands::Commit(args) => {
            commit::cmd_commit(args)?;
        }
        Commands::WriteTree => {
            write_tree::cmd_write_tree()?;
        }
    }

    Ok(())
}
