use crate::cli::{ObjectMode, RevParseArgs};
use crate::object::{ObjectKind, find_object};
use crate::repository::repo_find;
use std::io;
pub fn cmd_rev_parse(args: &RevParseArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let object_kind: Option<ObjectKind> = match args.object_type {
        Some(ObjectMode::Blob) => Some(ObjectKind::Blob),
        Some(ObjectMode::Commit) => Some(ObjectKind::Commit),
        Some(ObjectMode::Tag) => Some(ObjectKind::Tag),
        Some(ObjectMode::Tree) => Some(ObjectKind::Tree),
        None => None,
    };

    let result = find_object(&repo, &args.name, object_kind, true)?;

    println!("{}", result);

    Ok(())
}
