use crate::cli;
use crate::object::{self, GitObject};
use crate::repository::{GitRepository, repo_find};
use std::io::{self, Write};

pub fn cmd_cat_file(args: &cli::CatFileArgs) -> io::Result<()> {
    let repo: GitRepository = repo_find(None)?;
    let object_kind: object::ObjectKind = object::ObjectKind::new(args.mode);
    cat_file(&repo, &args.object, Some(object_kind))
}

fn cat_file(repo: &GitRepository, obj: &str, fmt: Option<object::ObjectKind>) -> io::Result<()> {
    let obj: GitObject =
        object::read_object(repo, &object::find_object(&repo, obj, fmt, false).as_str())?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&obj.serialize()?)?;
    Ok(())
}
