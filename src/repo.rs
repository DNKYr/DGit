use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use indexmap::{IndexMap, IndexSet};

use crate::cli;
use crate::object;
use crate::object::GitObject;

pub struct GitRepository {
    worktree: PathBuf,
    git_dir: PathBuf,
}

impl GitRepository {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            worktree: path.clone(),
            git_dir: path.clone().join(".git"),
        }
    }

    pub fn get_git_dir(&self) -> PathBuf {
        self.git_dir.clone()
    }
}

pub fn repo_path(repo: &GitRepository, paths: &[&str]) -> PathBuf {
    // Compute path under repo's gitdir
    let mut result: PathBuf = repo.get_git_dir();
    for path in paths {
        result = result.join(path);
    }
    result
}

pub fn repo_file(repo: &GitRepository, paths: &[&str], mkdir: Option<bool>) -> io::Result<PathBuf> {
    if paths.len() > 1 {
        let _ = repo_dir(repo, &paths[..paths.len() - 1], mkdir)?;
    }
    Ok(repo_path(repo, paths))
}

pub fn repo_dir(repo: &GitRepository, paths: &[&str], mkdir: Option<bool>) -> io::Result<PathBuf> {
    let path = repo_path(repo, paths);

    if path.exists() {
        if path.is_dir() {
            return Ok(path);
        } else {
            return Err(io::Error::other("Path exists but is not a directory"));
        }
    }

    if mkdir.unwrap_or(false) {
        fs::create_dir_all(&path)?;
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Directory does not exist",
        ))
    }
}

pub fn repo_create(path: &PathBuf) -> io::Result<String> {
    let repo: GitRepository = GitRepository::new(path);
    let git_dir_path: PathBuf = repo_path(&repo, &[]);
    let head_file_path: PathBuf = repo_path(&repo, &["HEAD"]);
    let ref_dir_path: PathBuf = repo_path(&repo, &["refs", "heads"]);
    let object_dir_path: PathBuf = repo_path(&repo, &["objects"]);

    fs::create_dir(git_dir_path)?;
    fs::create_dir(object_dir_path)?;
    fs::create_dir_all(ref_dir_path)?;
    fs::write(head_file_path, "ref: refs/heads/main")?;

    Ok(String::from("Initialized empty DGit repository"))
}

pub fn repo_find(path: Option<&PathBuf>) -> io::Result<GitRepository> {
    let current_directory = env::current_dir()?;
    let path: &Path = path
        .map(|p| p.as_path())
        .unwrap_or(current_directory.as_path());

    let git_dir_path: PathBuf = path.join(".git");

    if git_dir_path.exists() {
        return Ok(GitRepository::new(&path.to_path_buf()));
    }
    match path.parent() {
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            String::from("Not working within a Git repository"),
        )),

        Some(parent_path) => repo_find(Some(&parent_path.to_path_buf())),
    }
}

pub fn cmd_cat_file(args: &cli::CatFileArgs) -> io::Result<()> {
    let repo: GitRepository = repo_find(None)?;
    cat_file(&repo, &args.object, Some(args.mode))
}

fn cat_file(repo: &GitRepository, obj: &String, fmt: Option<cli::ObjectMode>) -> io::Result<()> {
    let obj: GitObject =
        object::read_object(repo, &object::find_object(&repo, obj, fmt, None).as_str())?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&obj.serialize()?)?;
    Ok(())
}

pub fn cmd_hash_object(args: &cli::HashObjectArgs) -> io::Result<()> {
    let sha: String = match args.write {
        true => hash_object(
            &PathBuf::from(&args.path),
            &args.types,
            Some(&repo_find(None)?),
        )?,

        false => hash_object(&PathBuf::from(&args.path), &args.types, None)?,
    };
    println!("{sha}");
    Ok(())
}

fn hash_object(
    file_path: &PathBuf,
    fmt: &cli::HashObjectType,
    repo: Option<&GitRepository>,
) -> io::Result<String> {
    let data: Vec<u8> = fs::read(file_path)?;
    let obj: GitObject = match fmt {
        cli::HashObjectType::Blob => object::GitObject::Blob(object::BlobObject::new(data)),
        _ => {
            unimplemented!("other three objects type");
        }
    };

    obj.write(repo)
}

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

pub fn cmd_ls_tree(args: &cli::LsTreeArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    ls_tree(&repo, &args.tree, args.recursive, None)
}

fn ls_tree(
    repo: &GitRepository,
    reference: &String,
    recursive: bool,
    prefix: Option<Vec<u8>>,
) -> io::Result<()> {
    let prefix = prefix.unwrap_or_default();

    let sha = object::find_object(repo, reference, Some(cli::ObjectMode::Tree), None);
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

    let sha = object::find_object(&repo, &args.commit, Some(cli::ObjectMode::Tree), None);
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

fn tree_checkout(
    repo: &GitRepository,
    tree: &object::TreeObject,
    path: &PathBuf,
) -> io::Result<()> {
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

fn ref_resolve(repo: &GitRepository, reference: &[&str]) -> io::Result<String> {
    let path: PathBuf = repo_file(repo, reference, None)?;
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

fn ref_list(repo: &GitRepository, path: Option<PathBuf>) -> io::Result<IndexMap<PathBuf, String>> {
    let path = path.unwrap_or(repo_dir(repo, &["refs"], None)?);

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

pub fn cmd_show_ref() -> io::Result<()> {
    let repo = repo_find(None)?;
    let refs = ref_list(&repo, None)?;
    show_ref(refs, None, None)
}

fn show_ref(
    refs: IndexMap<PathBuf, String>,
    with_hash: Option<bool>,
    prefix: Option<String>,
) -> io::Result<()> {
    let mut pr = String::new();
    if let Some(p) = prefix {
        pr = p + "/";
    }
    let with_hash = with_hash.unwrap_or(true);

    for (k, v) in refs.iter() {
        if with_hash {
            println!("{} {}{:?}", v, pr, k);
        } else {
            println!("{}{:?}", pr, k);
        }
    }

    Ok(())
}
