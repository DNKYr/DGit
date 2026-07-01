use crate::repository::{GitRepository, repo_path};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn init(path: &Path) -> io::Result<String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = repo_path(&repo, &[]);
    let head_file_path: PathBuf = repo_path(&repo, &["HEAD"]);
    let ref_dir_path: PathBuf = repo_path(&repo, &["refs", "heads"]);
    let object_dir_path: PathBuf = repo_path(&repo, &["objects"]);

    fs::create_dir(git_dir_path)?;
    fs::create_dir(object_dir_path)?;
    fs::create_dir_all(ref_dir_path)?;
    fs::write(head_file_path, "ref: refs/heads/main")?;

    Ok(String::from("Initialized empty dgit repository"))
}
