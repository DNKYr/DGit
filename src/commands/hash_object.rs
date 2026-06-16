use crate::cli;
use crate::object::{self, GitObject, write_object};
use crate::repository::{GitRepository, repo_find};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    file_path: &Path,
    fmt: &cli::HashObjectType,
    repo: Option<&GitRepository>,
) -> io::Result<String> {
    let data: Vec<u8> = fs::read(file_path)?;
    let obj: GitObject = match fmt {
        cli::HashObjectType::Blob => object::GitObject::Blob(object::BlobObject::new(data)),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "This object type isn't supported yet",
            ));
        }
    };

    write_object(&obj, repo)
}
