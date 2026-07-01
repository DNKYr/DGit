use crate::cli;
use crate::index;
use crate::repository::repo_find;
use sha1::{Digest, Sha1};
use std::fs;
use std::io;
use std::path::PathBuf;

pub fn cmd_rm(args: &cli::RmArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let mut index = index::read_index(&repo)?;

    let index_map: std::collections::BTreeMap<Vec<u8>, [u8; 20]> = index
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.sha))
        .collect();

    for file_path in &args.paths {
        let path_bytes = file_path.as_bytes().to_vec();

        let idx_sha = match index_map.get(&path_bytes) {
            Some(sha) => *sha,
            None => {
                eprintln!(
                    "error: pathspec '{}' did not match any files",
                    file_path
                );
                continue;
            }
        };

        let abs_path = PathBuf::from(file_path);
        let file_exists = abs_path.exists();

        if !args.cached && file_exists {
            if let Ok(worktree_sha) = file_blob_sha(&repo, &path_bytes) {
                if worktree_sha != idx_sha && !args.force {
                    eprintln!(
                        "error: '{}' has local modifications (use -f to override)",
                        file_path
                    );
                    continue;
                }
            }

            if let Err(e) = fs::remove_file(&abs_path) {
                eprintln!("error: cannot remove '{}': {}", file_path, e);
                continue;
            }
        }

        index.remove_entries_for_path(&path_bytes);
    }

    index::write_index(&repo, &index)?;

    Ok(())
}

fn file_blob_sha(repo: &crate::repository::GitRepository, path: &[u8]) -> io::Result<[u8; 20]> {
    let git_dir_path = repo.get_git_dir();
    let worktree =
        PathBuf::from(git_dir_path.parent().unwrap_or_else(|| std::path::Path::new(".")));
    let abs_path = worktree.join(String::from_utf8_lossy(path).as_ref());

    let data = fs::read(&abs_path)?;

    let size_bytes = data.len().to_string().into_bytes();
    let mut hasher = Sha1::new();
    hasher.update(b"blob ");
    hasher.update(&size_bytes);
    hasher.update(&[0u8]);
    hasher.update(&data);

    Ok(hasher.finalize().into())
}
