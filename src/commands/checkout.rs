use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::cli;
use crate::object::{self, GitObject, find_object};
use crate::repository::{GitRepository, repo_find};

pub fn cmd_checkout(args: &cli::CheckoutArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let path: PathBuf = PathBuf::from(&args.path);

    if path.exists() {
        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "Not a directory",
            ));
        }

        if fs::read_dir(&path)?.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::DirectoryNotEmpty,
                "Not an empty directory",
            ));
        }
    } else {
        fs::create_dir(&path)?;
    }

    let sha = find_object(&repo, &args.commit, Some(object::ObjectKind::Tree), false);
    let obj: object::TreeObject = match object::read_object(&repo, &sha)? {
        GitObject::Tree(tree) => tree,
        GitObject::Commit(tree) => {
            let sha_str = tree
                .get_kvlm()
                .get(&Some(b"tree".to_vec()))
                .and_then(|vec| vec.first())
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned());

            if let Some(sha) = sha_str {
                match object::read_object(&repo, &sha)? {
                    GitObject::Tree(obj) => obj,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "The commit object's tree object is not a tree object. (What?)",
                        ));
                    }
                }
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "The commit object has no tree object",
                ));
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The input hash isn't an Tree or a Commit object",
            ));
        }
    };

    tree_checkout(&repo, &obj, &path)
}

fn tree_checkout(repo: &GitRepository, tree: &object::TreeObject, path: &Path) -> io::Result<()> {
    for item in tree.get_items() {
        let sha_hex = &item
            .sha
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        let obj: GitObject = object::read_object(repo, sha_hex)?;
        let dest = path.join(String::from_utf8_lossy(&item.path).as_ref());

        match obj {
            GitObject::Tree(tree) => {
                fs::create_dir(&dest)?;
                tree_checkout(repo, &tree, &dest)?;
            }

            GitObject::Blob(blob) => {
                let mut file = fs::File::create(dest)?;
                file.write_all(&blob.data)?;
            }

            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Tree contains object other than blob and another tree",
                ));
            }
        }
    }
    Ok(())
}
