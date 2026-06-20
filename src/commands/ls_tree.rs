use std::io;

use crate::cli;
use crate::object;
use crate::object::GitObject;
use crate::repository::{GitRepository, repo_find};

pub fn cmd_ls_tree(args: &cli::LsTreeArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    ls_tree(&repo, &args.tree, args.recursive, None)
}

fn ls_tree(
    repo: &GitRepository,
    reference: &str,
    recursive: bool,
    prefix: Option<Vec<u8>>,
) -> io::Result<()> {
    let prefix = prefix.unwrap_or_default();

    let sha = object::find_object(repo, reference, Some(object::ObjectKind::Tree), true)?;
    let obj: object::TreeObject = match object::read_object(repo, &sha)? {
        GitObject::Tree(tree) => tree,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The reference is not an tree object",
            ));
        }
    };

    for item in obj.get_items() {
        let types = match item.mode.len() {
            5 => &item.mode[0..1],
            _ => &item.mode[0..2],
        };

        let types = match types {
            b"4" | b"04" => String::from("tree"),
            b"10" => String::from("blob"),
            b"12" => String::from("blob"),
            b"16" => String::from("commit"),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Weird tree leaf mode",
                ));
            }
        };

        let mut n_prefix = prefix.clone();
        if !n_prefix.is_empty() {
            n_prefix.push(b'/');
        }
        n_prefix.extend(&item.path);

        let sha_hex = &item
            .sha
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        if !(recursive && types == "tree") {
            println!(
                "{} {} {}  {}",
                String::from_utf8_lossy(&item.mode),
                types,
                sha_hex,
                String::from_utf8_lossy(&n_prefix),
            );
        } else {
            ls_tree(repo, sha_hex, recursive, Some(n_prefix.to_vec()))?
        }
    }
    Ok(())
}
