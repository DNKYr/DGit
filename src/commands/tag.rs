use indexmap::IndexMap;
use std::{fs, io};

use crate::cli;
use crate::object::{GitObject, TagObject, find_object, write_object};
use crate::refs::ref_list;
use crate::repository::{GitRepository, repo_dir, repo_file, repo_find};

pub fn cmd_tag(args: &cli::TagArgs) -> io::Result<()> {
    let repo = repo_find(None)?;

    if let Some(name) = &args.name {
        tag_create(&repo, name, &args.object, args.add)
    } else {
        let tags_dir = repo_dir(&repo, &["refs", "tags"], false)?;
        let refs = ref_list(&repo, Some(tags_dir.clone()))?;
        for path in refs.keys() {
            let name = path.strip_prefix(&tags_dir).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "tag path outside refs/tags")
            })?;

            println!("{}", name.display());
        }
        Ok(())
    }
}

fn tag_create(
    repo: &GitRepository,
    name: &str,
    refs: &str,
    create_tag_object: bool,
) -> io::Result<()> {
    let sha = find_object(repo, refs, None, false)?;

    if create_tag_object {
        let sha = sha.as_bytes();
        let byte_name = name.as_bytes();
        let mut kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>> = IndexMap::new();
        kvlm.insert(Some(b"object".to_vec()), vec![sha.to_vec()]);
        kvlm.insert(Some(b"type".to_vec()), vec![b"commit".to_vec()]);
        kvlm.insert(Some(b"tag".to_vec()), vec![byte_name.to_vec()]);
        kvlm.insert(
            Some(b"tagger".to_vec()),
            vec![b"DGit <dgit@example.com>".to_vec()],
        );
        kvlm.insert(None, vec![b"A tag created by DGit, can't modify".to_vec()]);
        let tag = GitObject::Tag(TagObject::new(kvlm));
        let tag_sha = write_object(&tag, Some(repo))?;
        ref_create(repo, &["refs", "tags", name], &tag_sha)
    } else {
        ref_create(repo, &["refs", "tags", name], &sha)
    }
}

fn ref_create(repo: &GitRepository, ref_name: &[&str], sha: &str) -> io::Result<()> {
    let path = repo_file(repo, ref_name, true)?;

    fs::write(path, format!("{}\n", sha))?;
    Ok(())
}
