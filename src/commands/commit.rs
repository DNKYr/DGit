use crate::cli;
use crate::commands::write_tree;
use crate::config::Config;
use crate::object::{self, CommitObject, GitObject, KvlmMap};
use crate::refs;
use crate::repository::repo_find;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn cmd_commit(args: &cli::CommitArgs) -> io::Result<()> {
    let repo = repo_find(None)?;

    let branch = match refs::get_active_branch(&repo)? {
        Some(b) => b,
        None => {
            return Err(io::Error::other(
                "HEAD is detached; cannot commit",
            ));
        }
    };

    let config = Config::read(&repo)?;

    let user_name = config
        .get("user", "name")
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GIT_AUTHOR_NAME").ok())
        .ok_or_else(|| {
            io::Error::other(
                "user.name not set in config and GIT_AUTHOR_NAME not set",
            )
        })?;

    let user_email = config
        .get("user", "email")
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GIT_AUTHOR_EMAIL").ok())
        .ok_or_else(|| {
            io::Error::other(
                "user.email not set in config and GIT_AUTHOR_EMAIL not set",
            )
        })?;

    let tree_sha = write_tree::tree_sha_from_index(&repo)?;

    let mut kvlm = KvlmMap::new();

    kvlm.insert(Some(b"tree".to_vec()), vec![tree_sha.into_bytes()]);

    if let Ok(parent_sha) = refs::ref_resolve(&repo, &["HEAD"]) {
        kvlm.insert(Some(b"parent".to_vec()), vec![parent_sha.into_bytes()]);
    }

    let ts = timestamp();

    let author = format!("{} <{}> {}", user_name, user_email, ts);
    let committer = format!("{} <{}> {}", user_name, user_email, ts);

    kvlm.insert(Some(b"author".to_vec()), vec![author.into_bytes()]);
    kvlm.insert(
        Some(b"committer".to_vec()),
        vec![committer.into_bytes()],
    );
    kvlm.insert(None, vec![args.message.as_bytes().to_vec()]);

    let commit = GitObject::Commit(CommitObject::new(kvlm));
    let sha = object::write_object(&commit, Some(&repo))?;

    refs::ref_write(&repo, &["refs", "heads", &branch], &sha)?;

    println!("[{} {:7}] {}", branch, &sha[..7], args.message);

    Ok(())
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{} +0000", now.as_secs())
}
