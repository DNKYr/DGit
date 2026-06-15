use std::io;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub struct GitRepository {
    worktree: PathBuf,
    git_dir: PathBuf,
}

impl GitRepository {
    pub fn new(path: &Path) -> Self {
        Self {
            worktree: path.to_owned().clone(),
            git_dir: path.join(".git"),
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

pub fn repo_file(repo: &GitRepository, paths: &[&str], mkdir: bool) -> io::Result<PathBuf> {
    if paths.len() > 1 {
        let _ = repo_dir(repo, &paths[..paths.len() - 1], mkdir)?;
    }
    Ok(repo_path(repo, paths))
}

pub fn repo_dir(repo: &GitRepository, paths: &[&str], mkdir: bool) -> io::Result<PathBuf> {
    let path = repo_path(repo, paths);

    if path.exists() {
        if path.is_dir() {
            return Ok(path);
        } else {
            return Err(io::Error::other("Path exists but is not a directory"));
        }
    }

    if mkdir {
        fs::create_dir_all(&path)?;
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Directory does not exist",
        ))
    }
}

pub fn repo_find(path: Option<&Path>) -> io::Result<GitRepository> {
    let current_directory = env::current_dir()?;
    let path: &Path = path.unwrap_or(current_directory.as_path());

    let git_dir_path: PathBuf = path.join(".git");

    if git_dir_path.exists() {
        return Ok(GitRepository::new(path));
    }
    match path.parent() {
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            String::from("Not working within a Git repository"),
        )),

        Some(parent_path) => repo_find(Some(parent_path)),
    }
}
