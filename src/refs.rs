use crate::repository::{GitRepository, repo_dir, repo_file};
use indexmap::IndexMap;
use std::fs;
use std::io;
use std::path::PathBuf;

pub fn ref_resolve(repo: &GitRepository, reference: &[&str]) -> io::Result<String> {
    let path: PathBuf = repo_file(repo, reference, false)?;
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the path to the store reference is empty. Possibility due to this is an new repository with no commits",
        ));
    }

    let data: String = fs::read_to_string(path)?;
    let data = data.trim();

    if let Some(target_path) = data.strip_prefix("ref: ") {
        let next_ref: Vec<&str> = target_path.split("/").collect();
        ref_resolve(repo, &next_ref)
    } else {
        Ok(data.to_string())
    }
}

pub fn ref_list(
    repo: &GitRepository,
    path: Option<PathBuf>,
) -> io::Result<IndexMap<PathBuf, String>> {
    let path = path.unwrap_or(repo_dir(repo, &["refs"], false)?);

    let mut ret: IndexMap<PathBuf, String> = IndexMap::new();

    let directory = fs::read_dir(&path)?;

    for content in directory {
        let entry_path = content?.path();
        let next: PathBuf = path.join(entry_path);
        if next.is_dir() {
            let submap = ref_list(repo, Some(next))?;
            ret.extend(submap);
        } else {
            let rel = next.strip_prefix(repo.get_git_dir()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ref path outside of git directory",
                )
            })?;

            let hold: Vec<&str> = rel
                .iter()
                .map(|s| {
                    s.to_str().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in path")
                    })
                })
                .collect::<io::Result<_>>()?;

            let sha = ref_resolve(repo, &hold)?;

            ret.insert(next, sha);
        }
    }
    Ok(ret)
}
