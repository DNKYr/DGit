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

    pub fn get_git_dir(&self) -> PathBuf {
        self.git_dir.clone()
    }
}

pub fn repo_path(repo: &GitRepository, paths: &[&str]) -> PathBuf {
    // Compute path under repo's gitdir
    let mut result: PathBuf = repo.get_git_dir();
    for path in paths {
        result = result.join(path);
    }
    result
}

pub fn repo_create(path: &PathBuf) -> Result<String, String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = repo_path(&repo, &[]);
    let head_file_path: PathBuf = repo_path(&repo, &["HEAD"]);
    let ref_dir_path: PathBuf = repo_path(&repo, &["refs", "heads"]);
    let object_dir_path: PathBuf = repo_path(&repo, &["objects"]);

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

pub fn repo_find(path: &PathBuf) -> Result<GitRepository, String> {
    let git_dir_path: PathBuf = path.join(".git");
    if git_dir_path.exists() {
        return Ok(GitRepository::new(&path));
    }
    match path.parent() {
        None => {
            return Err(String::from("Not working within a Git repository"));
        }

        Some(_) => {
            return repo_find(&path.parent().unwrap().to_path_buf());
        }
    }
}
