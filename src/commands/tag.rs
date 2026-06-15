use std::{fs, io};

use crate::cli;
use crate::commands::show_ref::show_ref;
use crate::object;
use crate::refs::ref_list;
use crate::repository::{GitRepository, repo_file, repo_find};

pub fn cmd_tag(args: &cli::TagArgs) -> io::Result<()> {
    let repo = repo_find(None)?;

    if let Some(name) = &args.name {
        tag_create(&repo, name, &args.object, args.add)
    } else {
        let refs = ref_list(&repo, None)?;
        let prefix: String = refs
            .get(&repo.get_git_dir())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Cannot find refs directory in .git directory",
                )
            })?
            .clone();
        show_ref(refs, false, Some(prefix))
    }
}

fn tag_create(
    repo: &GitRepository,
    name: &str,
    refs: &str,
    create_tag_object: bool,
) -> io::Result<()> {
    let sha = object::find_object(repo, name, None, false);

    ref_create(repo, &["refs", "tags", name], &sha)
}

fn ref_create(repo: &GitRepository, ref_name: &[&str], sha: &str) -> io::Result<()> {
    let path = repo_file(repo, ref_name, false)?;

    fs::write(path, format!("{}\n", sha));
    Ok(())
}
