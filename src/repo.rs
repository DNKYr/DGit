use std::fs;
use std::io;
use std::path::PathBuf;
use std::path::{Path, PathBuf};

use crate::object;

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

pub fn repo_file(repo: &GitRepository, paths: &[&str], mkdir: Option<bool>) -> io::Result<PathBuf> {
    if paths.len() > 1 {
        let _ = repo_dir(repo, &paths[..paths.len() - 1], mkdir);
    }
    Ok(repo_path(repo, paths))
}

pub fn repo_dir(repo: &GitRepository, paths: &[&str], mkdir: Option<bool>) -> io::Result<PathBuf> {
    let path = repo_path(repo, paths);

    if path.exists() {
        if path.is_dir() {
            return Ok(path);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Path exists but is not a directory",
            ));
        }
    }

    if mkdir.unwrap_or(false) {
        fs::create_dir_all(&path)?;
        return Ok(path);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Directory does not exist",
        ));
    }
}

pub fn repo_create(path: &PathBuf) -> io::Result<String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = repo_path(&repo, &[]);
    let head_file_path: PathBuf = repo_path(&repo, &["HEAD"]);
    let ref_dir_path: PathBuf = repo_path(&repo, &["refs", "heads"]);
    let object_dir_path: PathBuf = repo_path(&repo, &["objects"]);

    fs::create_dir(git_dir_path)?;
    fs::create_dir(object_dir_path)?;
    fs::create_dir_all(ref_dir_path)?;
    fs::write(head_file_path, "ref: refs/heads/main")?;

    Ok(String::from("Initialized empty DGit repository"))
}

pub fn repo_find(path: Option<&PathBuf>) -> Result<GitRepository, String> {
    let path: &Path = path.map(|p| p.as_path()).unwrap_or(Path::new("."));
    let git_dir_path: PathBuf = path.join(".git");
    if git_dir_path.exists() {
        return Ok(GitRepository::new(&path.to_path_buf()));
    }
    match path.parent() {
        None => {
            return Err(String::from("Not working within a Git repository"));
        }

        Some(_) => {
            return repo_find(Some(&path.parent().unwrap().to_path_buf()));
        }
    }
}
