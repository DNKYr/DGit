use crate::gitignore::IgnoreRules;
use crate::index;
use crate::object::{self, GitObject};
use crate::refs;
use crate::repository::{GitRepository, repo_find};
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;

pub fn cmd_status() -> io::Result<()> {
    let repo = repo_find(None)?;
    let index = index::read_index(&repo)?;
    let ignore_rules = IgnoreRules::load(&repo)?;

    let head_tree = read_head_tree_flat(&repo)?;

    let index_map: BTreeMap<Vec<u8>, (u32, [u8; 20])> = index
        .entries
        .iter()
        .map(|e| (e.path.clone(), (e.mode, e.sha)))
        .collect();

    let index_paths: BTreeSet<Vec<u8>> = index_map.keys().cloned().collect();
    let head_paths: BTreeSet<Vec<u8>> = head_tree
        .as_ref()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let worktree_files = walk_worktree(&repo)?;

    let mut staged_new: Vec<&[u8]> = Vec::new();
    let mut staged_modified: Vec<&[u8]> = Vec::new();
    let mut staged_deleted: Vec<&[u8]> = Vec::new();
    let mut unstaged_modified: Vec<&[u8]> = Vec::new();
    let mut unstaged_deleted: Vec<&[u8]> = Vec::new();
    let mut untracked: Vec<&[u8]> = Vec::new();

    if let Some(ref head) = head_tree {
        for path in &head_paths {
            if let Some(&(_, idx_sha)) = index_map.get(path) {
                if let Some((_, head_sha)) = head.get(path)
                    && idx_sha.as_slice() != head_sha.as_slice() {
                        staged_modified.push(path);
                    }
            } else {
                staged_deleted.push(path);
            }
        }
    }

    for path in &index_paths {
        if let Some(ref head) = head_tree {
            if !head.contains_key(path) {
                staged_new.push(path);
            }
        } else {
            staged_new.push(path);
        }

        if worktree_files.contains(path) {
            let worktree_sha = file_blob_sha(&repo, path)?;
            let &(_, idx_sha) = index_map.get(path).unwrap();
            if worktree_sha != idx_sha {
                unstaged_modified.push(path);
            }
        } else {
            unstaged_deleted.push(path);
        }
    }

    for path in &worktree_files {
        if !index_map.contains_key(path) && !ignore_rules.is_ignored(path, false) {
            untracked.push(path);
        }
    }

    staged_new.sort();
    staged_modified.sort();
    staged_deleted.sort();
    unstaged_modified.sort();
    unstaged_deleted.sort();
    untracked.sort();

    let has_staged = !staged_new.is_empty() || !staged_modified.is_empty() || !staged_deleted.is_empty();
    let has_unstaged = !unstaged_modified.is_empty() || !unstaged_deleted.is_empty();
    let has_untracked = !untracked.is_empty();

    if has_staged {
        println!("Changes to be committed:");
        println!("  (use \"dgit add <file>...\" to update what will be committed)");
        for p in &staged_new {
            println!("\tnew file:\t{}", String::from_utf8_lossy(p));
        }
        for p in &staged_modified {
            println!("\tmodified:\t{}", String::from_utf8_lossy(p));
        }
        for p in &staged_deleted {
            println!("\tdeleted:\t{}", String::from_utf8_lossy(p));
        }
        if has_unstaged || has_untracked {
            println!();
        }
    }

    if has_unstaged {
        println!("Changes not staged for commit:");
        println!("  (use \"dgit add <file>...\" to update what will be committed)");
        for p in &unstaged_modified {
            println!("\tmodified:\t{}", String::from_utf8_lossy(p));
        }
        for p in &unstaged_deleted {
            println!("\tdeleted:\t{}", String::from_utf8_lossy(p));
        }
        if has_untracked {
            println!();
        }
    }

    if has_untracked {
        println!("Untracked files:");
        println!("  (use \"dgit add <file>...\" to include in what will be committed)");
        for p in &untracked {
            println!("\t{}", String::from_utf8_lossy(p));
        }
    }

    if !has_staged && !has_unstaged && !has_untracked {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
}

type FlatTree = BTreeMap<Vec<u8>, (Vec<u8>, Vec<u8>)>;

fn read_head_tree_flat(
    repo: &GitRepository,
) -> io::Result<Option<FlatTree>> {
    let head_sha = match refs::ref_resolve(repo, &["HEAD"]) {
        Ok(sha) => sha,
        Err(_) => return Ok(None),
    };

    let commit = match object::read_object(repo, &head_sha)? {
        GitObject::Commit(c) => c,
        _ => return Ok(None),
    };

    let tree_sha = match commit
        .get_kvlm()
        .get(&Some(b"tree".to_vec()))
        .and_then(|v| v.first())
        .map(|b| String::from_utf8_lossy(b).into_owned())
    {
        Some(sha) => sha,
        None => return Ok(None),
    };

    let tree = match object::read_object(repo, &tree_sha)? {
        GitObject::Tree(t) => t,
        _ => return Ok(None),
    };

    let mut map = BTreeMap::new();
    flatten_tree(repo, &tree, &[], &mut map)?;
    Ok(Some(map))
}

fn flatten_tree(
    repo: &GitRepository,
    tree: &object::TreeObject,
    prefix: &[u8],
    map: &mut BTreeMap<Vec<u8>, (Vec<u8>, Vec<u8>)>,
) -> io::Result<()> {
    for leaf in tree.get_items() {
        let mut full_path = prefix.to_vec();
        full_path.extend_from_slice(&leaf.path);

        if leaf.mode.starts_with(b"4") {
            let sha_hex: String = leaf.sha.iter().map(|b| format!("{:02x}", b)).collect();
            let sub_tree = match object::read_object(repo, &sha_hex)? {
                GitObject::Tree(t) => t,
                _ => continue,
            };
            let mut sub_prefix = full_path;
            sub_prefix.push(b'/');
            flatten_tree(repo, &sub_tree, &sub_prefix, map)?;
        } else {
            map.insert(full_path, (leaf.mode.clone(), leaf.sha.clone()));
        }
    }
    Ok(())
}

fn walk_worktree(repo: &GitRepository) -> io::Result<BTreeSet<Vec<u8>>> {
    let mut files = BTreeSet::new();
    let git_dir_path = repo.get_git_dir();
    let worktree = std::path::PathBuf::from(git_dir_path.parent().unwrap_or_else(|| {
        std::path::Path::new(".")
    }));

    if !worktree.exists() {
        return Ok(files);
    }

    walk_dir(&worktree, &[], &mut files)?;
    Ok(files)
}

fn walk_dir(
    base: &std::path::Path,
    prefix: &[u8],
    files: &mut BTreeSet<Vec<u8>>,
) -> io::Result<()> {
    let dir = if prefix.is_empty() {
        base.to_path_buf()
    } else {
        base.join(String::from_utf8_lossy(prefix).as_ref())
    };

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name_bytes = entry.file_name().as_encoded_bytes().to_vec();

        if name_bytes == b".git" {
            continue;
        }

        let mut rel_path = prefix.to_vec();
        if !rel_path.is_empty() {
            rel_path.push(b'/');
        }
        rel_path.extend_from_slice(&name_bytes);

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            walk_dir(base, &rel_path, files)?;
        } else {
            files.insert(rel_path);
        }
    }

    Ok(())
}

fn file_blob_sha(repo: &GitRepository, path: &[u8]) -> io::Result<[u8; 20]> {
    let git_dir_path = repo.get_git_dir();
    let worktree = std::path::PathBuf::from(git_dir_path.parent().unwrap_or_else(|| {
        std::path::Path::new(".")
    }));
    let abs_path = worktree.join(String::from_utf8_lossy(path).as_ref());

    let data = fs::read(&abs_path)?;

    let size_bytes = data.len().to_string().into_bytes();
    let mut hasher = Sha1::new();
    hasher.update(b"blob ");
    hasher.update(&size_bytes);
    hasher.update([0u8]);
    hasher.update(&data);

    Ok(hasher.finalize().into())
}
