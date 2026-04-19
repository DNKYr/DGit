use std::fs;
use std::path::PathBuf;

pub struct GitRepository {
    worktree: PathBuf,
    git_dir: PathBuf,
}

impl GitRepository {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            worktree: path.clone(),
            git_dir: path.clone().join(".git"),
        }
    }

    pub fn get_git_dir(self) -> PathBuf {
        self.git_dir.clone()
    }
}

pub fn repo_create(path: &PathBuf) -> Result<String, String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = PathBuf::from(repo.get_git_dir());
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

