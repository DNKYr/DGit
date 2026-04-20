mod repo;

use std::env;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let len_args = args.len();

    match len_args {
        0 => {
            println!("Invalid to input 0 arguments");
            process::exit(1);
        }
        _ => {}
    }

    let command = &args[1];
    let current_directory_path = env::current_dir()?;

    match command.as_str() {
        "init" => match repo::repo_create(&current_directory_path) {
            Ok(success_msg) => {
                println!("{success_msg}")
            }
            Err(err_msg) => {
                println!("{err_msg}");
            }
        },

        "status" => match repo::repo_find(&current_directory_path) {
            Ok(repo) => {
                println!("{:?}", repo.get_git_dir().display());
            }

            Err(err_msg) => {
                println!("{err_msg}");
                process::exit(1);
            }
        },

        "hash-object" => {}

        _ => {
            println!("Invalid command");
        }
    }

    Ok(())
}
