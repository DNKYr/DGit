use crate::repository::{GitRepository, repo_dir, repo_file};
use indexmap::IndexMap;
use std::fs;
use std::io;
use std::io::Write;
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

pub fn get_active_branch(repo: &GitRepository) -> io::Result<Option<String>> {
    let head_path = repo_file(repo, &["HEAD"], false)?;

    if !head_path.is_file() {
        return Ok(None);
    }

    let data = fs::read_to_string(&head_path)?;
    let data = data.trim();

    if let Some(target) = data.strip_prefix("ref: refs/heads/") {
        Ok(Some(target.to_string()))
    } else if data.starts_with("ref: ") {
        Ok(None)
    } else {
        Ok(None)
    }
}

pub fn ref_write(repo: &GitRepository, reference: &[&str], sha: &str) -> io::Result<()> {
    let path = repo_file(repo, reference, true)?;
    let mut file = fs::File::create(path)?;
    file.write_all(sha.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo(name: &str) -> (PathBuf, GitRepository) {
        let root = std::env::temp_dir().join(name);
        let git_dir = root.join(".git");

        std::fs::create_dir_all(git_dir.join("refs").join("heads")).unwrap();
        std::fs::create_dir_all(git_dir.join("refs").join("tags")).unwrap();

        let repo = GitRepository::new(&root);

        (root, repo)
    }

    #[test]
    fn ref_resolve_reads_direct_ref() {
        let (root, repo) = test_repo("dgit-refs-test-direct-ref");
        let sha = "0123456789abcdef0123456789abcdef01234567";
        std::fs::write(
            root.join(".git").join("refs").join("heads").join("main"),
            sha,
        )
        .unwrap();

        let resolved = ref_resolve(&repo, &["refs", "heads", "main"]).unwrap();

        assert_eq!(resolved, sha);
    }

    #[test]
    fn ref_resolve_follows_symbolic_ref() {
        let (root, repo) = test_repo("dgit-refs-test-symbolic-ref");
        let sha = "fedcba9876543210fedcba9876543210fedcba98";
        std::fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();
        std::fs::write(
            root.join(".git").join("refs").join("heads").join("main"),
            sha,
        )
        .unwrap();

        let resolved = ref_resolve(&repo, &["HEAD"]).unwrap();

        assert_eq!(resolved, sha);
    }

    #[test]
    fn ref_resolve_errors_for_missing_ref() {
        let (_, repo) = test_repo("dgit-refs-test-missing-ref");

        let err = ref_resolve(&repo, &["refs", "heads", "missing"]).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn ref_list_lists_refs_recursively() {
        let (root, repo) = test_repo("dgit-refs-test-list");
        let main_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let tag_sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let main_ref = root.join(".git").join("refs").join("heads").join("main");
        let tag_ref = root.join(".git").join("refs").join("tags").join("v1.0");

        std::fs::write(&main_ref, main_sha).unwrap();
        std::fs::write(&tag_ref, tag_sha).unwrap();

        let refs = ref_list(&repo, None).unwrap();

        assert_eq!(refs.len(), 2);
        assert_eq!(refs.get(&main_ref).unwrap(), main_sha);
        assert_eq!(refs.get(&tag_ref).unwrap(), tag_sha);
    }

    #[test]
    fn get_active_branch_returns_branch_name() {
        let (root, repo) = test_repo("dgit-refs-test-active-branch");
        std::fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();

        let branch = get_active_branch(&repo).unwrap();
        assert_eq!(branch, Some("main".to_string()));
    }

    #[test]
    fn get_active_branch_returns_none_for_detached_head() {
        let (root, repo) = test_repo("dgit-refs-test-detached-head");
        std::fs::write(
            root.join(".git").join("HEAD"),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        let branch = get_active_branch(&repo).unwrap();
        assert_eq!(branch, None);
    }

    #[test]
    fn ref_write_creates_file_with_sha() {
        let (root, repo) = test_repo("dgit-refs-test-ref-write");
        let sha = "cccccccccccccccccccccccccccccccccccccccc";
        ref_write(&repo, &["refs", "heads", "feature"], sha).unwrap();

        let path = root
            .join(".git")
            .join("refs")
            .join("heads")
            .join("feature");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.trim(), sha);
    }

    #[test]
    fn ref_write_creates_parent_directories() {
        let (root, repo) = test_repo("dgit-refs-test-ref-write-dirs");
        let sha = "dddddddddddddddddddddddddddddddddddddddd";
        ref_write(&repo, &["refs", "remotes", "origin", "HEAD"], sha).unwrap();

        let path = root
            .join(".git")
            .join("refs")
            .join("remotes")
            .join("origin")
            .join("HEAD");
        assert!(path.is_file());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.trim(), sha);
    }
}
