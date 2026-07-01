use crate::index::{self, IndexEntry};
use crate::object::{self, GitObject, TreeLeaf, TreeObject};
use crate::repository::repo_find;
use std::collections::BTreeMap;
use std::io;

pub fn cmd_write_tree() -> io::Result<()> {
    let repo = repo_find(None)?;
    let index = index::read_index(&repo)?;

    let entries: Vec<(Vec<u8>, &IndexEntry)> = index
        .entries
        .iter()
        .map(|e| (e.path.clone(), e))
        .collect();

    let sha = write_tree_recursive(&repo, &entries)?;
    println!("{sha}");
    Ok(())
}

fn write_tree_recursive(
    repo: &crate::repository::GitRepository,
    entries: &[(Vec<u8>, &IndexEntry)],
) -> io::Result<String> {
    let mut groups: BTreeMap<Vec<u8>, Vec<(Vec<u8>, [u8; 20], u32)>> = BTreeMap::new();

    for (path, entry) in entries {
        if let Some(slash_pos) = path.iter().position(|&b| b == b'/') {
            let dir = path[..slash_pos].to_vec();
            let rest = path[slash_pos + 1..].to_vec();
            groups
                .entry(dir)
                .or_default()
                .push((rest, entry.sha, entry.mode));
        } else {
            groups
                .entry(path.clone())
                .or_default()
                .push((vec![], entry.sha, entry.mode));
        }
    }

    let mut items = Vec::new();

    for (name, children) in &groups {
        // A group with a single child having empty remainder is a file
        if children.len() == 1 && children[0].0.is_empty() {
            let (_, sha, mode) = children[0];
            let mode_str = format!("{:o}", mode).into_bytes();
            items.push(TreeLeaf::new(mode_str, name.clone(), sha.to_vec()));
        } else {
            // A directory: recursively process
            let sub_entries: Vec<(Vec<u8>, IndexEntry)> = children
                .iter()
                .filter(|(rest, _, _)| !rest.is_empty())
                .map(|(rest, sha, mode)| {
                    let entry = IndexEntry::new(rest.clone(), *sha, *mode, 0);
                    (rest.clone(), entry)
                })
                .collect();

            let sha_hex = write_tree_recursive(repo, &sub_entries.iter().map(|(p, e)| (p.clone(), e)).collect::<Vec<_>>())?;
            let sha_raw = hex::decode(&sha_hex)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            items.push(TreeLeaf::new(b"40000".to_vec(), name.clone(), sha_raw));
        }
    }

    let tree = GitObject::Tree(TreeObject::new(items));
    object::write_object(&tree, Some(repo))
}
