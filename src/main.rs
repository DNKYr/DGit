use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

struct GitRepository {
    worktree: PathBuf,
    git_dir: PathBuf,
}

impl GitRepository {
    fn new(path: &PathBuf) -> Self {
        Self {
            worktree: path.clone(),
            git_dir: path.clone().join(".git"),
        }
    }
}

fn repo_create(path: &PathBuf) -> Result<String, String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = PathBuf::from(&repo.git_dir);
    let head_file_path: PathBuf = PathBuf::from(&git_dir_path).join("HEAD");
    let ref_dir_path: PathBuf = PathBuf::from(&git_dir_path).join("refs/heads");
    let object_dir_path: PathBuf = PathBuf::from(&git_dir_path).join("objects");

    match fs::create_dir(git_dir_path) {
        Ok(v) => {
            fs::create_dir(object_dir_path);
            fs::create_dir_all(ref_dir_path);
            fs::write(head_file_path, "ref: refs/heads/main");
        }
        Err(e) => {
            println!("{e}");
            return Err(String::from(
                "Error, initializing on an existent git repository",
            ));
        }
    }

    Ok(String::from("Initialized empty DGit repository"))
}

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
        "init" => match repo_create(&current_directory_path) {
            Ok(success_msg) => {
                println!("{success_msg}")
            }
            Err(err_msg) => {
                println!("{err_msg}");
            }
        },

        _ => {
            println!("Invalid command");
        }
    }

    Ok(())
}
