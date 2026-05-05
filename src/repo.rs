use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use indexmap::IndexSet;

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
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Path exists but is not a directory",
            ));
        }
    }

    if mkdir.unwrap_or(false) {
        fs::create_dir_all(&path)?;
        return Ok(path);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Directory does not exist",
        ));
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
        .unwrap_or(&current_directory.as_path());

    let git_dir_path: PathBuf = path.join(".git");

    if git_dir_path.exists() {
        return Ok(GitRepository::new(&path.to_path_buf()));
    }
    match path.parent() {
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                String::from("Not working within a Git repository"),
            ));
        }

        Some(parent_path) => {
            return repo_find(Some(&parent_path.to_path_buf()));
        }
    }
}

pub fn cmd_cat_file(args: &cli::CatFileArgs) -> io::Result<()> {
    let repo: GitRepository = repo_find(None)?;
    cat_file(&repo, &args.object, Some(args.mode))
}

fn cat_file(repo: &GitRepository, obj: &String, fmt: Option<cli::CatFileMode>) -> io::Result<()> {
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
