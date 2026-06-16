use crate::cli::{CatFileArgs, ObjectMode};
use crate::object::{GitObject, ObjectKind, find_object, read_object};
use crate::repository::{GitRepository, repo_find};
use std::io::{self, Write};

pub fn cmd_cat_file(args: &CatFileArgs) -> io::Result<()> {
    let repo: GitRepository = repo_find(None)?;
    let object_kind: ObjectKind = match args.mode {
        ObjectMode::Blob => ObjectKind::Blob,
        ObjectMode::Commit => ObjectKind::Commit,
        ObjectMode::Tag => ObjectKind::Tag,
        ObjectMode::Tree => ObjectKind::Tree,
    };
    cat_file(&repo, &args.object, Some(object_kind))
}

fn cat_file(repo: &GitRepository, obj: &str, fmt: Option<ObjectKind>) -> io::Result<()> {
    let obj: GitObject = read_object(repo, &find_object(&repo, obj, fmt, false).as_str())?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&obj.serialize()?)?;
    Ok(())
}
