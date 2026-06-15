use std::io;

use indexmap::IndexSet;

use crate::cli;
use crate::object;
use crate::object::GitObject;
use crate::repository::{GitRepository, repo_find};

pub fn cmd_log(args: &cli::LogArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let mut seen_set: IndexSet<String> = IndexSet::new();

    println!("digraph DGitlog{{");
    println!("  node[shape=rect]");
    log_graphviz(
        &repo,
        args.commit_hash.clone().unwrap_or(String::from("HEAD")),
        &mut seen_set,
    )?;
    println!("}}");
    Ok(())
}

fn log_graphviz(repo: &GitRepository, sha: String, seen: &mut IndexSet<String>) -> io::Result<()> {
    if seen.contains(&sha) {
        return Ok(());
    }

    seen.insert(sha.clone());

    let commit: object::CommitObject = match object::read_object(repo, &sha)? {
        GitObject::Commit(commit) => commit,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The underlying hash is not a commit object",
            ));
        }
    };

    let kvlm = commit.get_kvlm();
    let raw_data = kvlm
        .get(&None)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No message found"))?;

    // Flatten the commit object and extract the message to string and handle error
    let message: String = String::from_utf8(raw_data.first().cloned().unwrap_or_default())
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Commit log contains non-UTF-8 characters",
            )
        })?
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n");

    let short = sha.get(..7).unwrap_or(sha.as_str());
    println!("c_{} [label=\"{}: {}\"]", sha, short, message);

    let Some(parents) = kvlm.get(&Some(b"parent".to_vec())) else {
        return Ok(());
    };

    for p in parents {
        let decode_p = match String::from_utf8(p.clone()) {
            Ok(d) => d,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Parent Commit log contains non-UTF-8 characters",
                ));
            }
        };
        println!("c_{} -> c_{}", sha, decode_p);
        log_graphviz(repo, decode_p, seen)?;
    }
    Ok(())
}
