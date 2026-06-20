use super::ObjectKind;
use crate::object::{GitObject, read_object};
use crate::refs::ref_resolve;
use crate::repository::{GitRepository, repo_dir};
use std::{fs, io};
fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}
fn resolve_object(repo: &GitRepository, name: &str) -> io::Result<String> {
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The name for an object cannot be empty",
        ));
    }

    if name == "HEAD" {
        return ref_resolve(repo, &["HEAD"]);
    }

    let mut candidates = Vec::new();

    if is_hex(name) && name.len() >= 2 {
        let name = name.to_lowercase();
        let prefix = &name[0..2];
        let rem = &name[2..];
        if let Ok(path) = repo_dir(repo, &["objects", prefix], false) {
            if let Ok(directory) = fs::read_dir(path) {
                for content in directory {
                    let dir_name = content?.file_name().into_string().map_err(|_| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Cant convert OsString to String",
                        )
                    })?;
                    if dir_name.starts_with(rem) {
                        candidates.push(prefix.to_owned() + &dir_name)
                    }
                }
            }
        }
    }
    // Try for reference
    if let Ok(as_tag) = ref_resolve(repo, &["refs", "tags", name]) {
        candidates.push(as_tag);
    }

    // try for branch
    if let Ok(as_branch) = ref_resolve(repo, &["refs", "heads", name]) {
        candidates.push(as_branch);
    }
    // try for remote branch
    if let Ok(as_remote_branch) = ref_resolve(repo, &["refs", "remotes", name]) {
        candidates.push(as_remote_branch);
    }

    match candidates.len() {
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No such a reference {name}"),
        )),
        1 => Ok(candidates[0].clone()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "ambiguous reference {name}: Candidates are\n - {:?}",
                candidates
            ),
        )),
    }
}

pub fn find_object(
    repo: &GitRepository,
    name: &str,
    fmt: Option<ObjectKind>,
    follow: bool,
) -> io::Result<String> {
    let mut sha = resolve_object(repo, name)?;
    let fmt = match fmt {
        Some(fmt) => fmt,
        None => return Ok(sha),
    };

    loop {
        let obj = read_object(repo, &sha)?;

        if obj.kind() == &fmt {
            return Ok(sha);
        }

        if !follow {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("cannot find {fmt:?} from {name}"),
            ));
        }

        sha = match (obj, &fmt) {
            (GitObject::Tag(tag), _) => {
                let object = tag
                    .get_kvlm()
                    .get(&Some(b"object".to_vec()))
                    .and_then(|v| v.first())
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "tag object has no object field")
                    })?;
                String::from_utf8(object.clone()).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "tag object field is not UTF-8")
                })?
            }
            (GitObject::Commit(commit), ObjectKind::Tree) => {
                let tree = commit
                    .get_kvlm()
                    .get(&Some(b"tree".to_vec()))
                    .and_then(|v| v.first())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "commit object has no tree field",
                        )
                    })?;
                String::from_utf8(tree.clone()).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "commit tree field is not UTF-8")
                })?
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("cannot find {fmt:?} from {name}"),
                ));
            }
        }
    }
}
