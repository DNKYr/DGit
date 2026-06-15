use std::io;
use std::path::PathBuf;

use indexmap::IndexMap;

use crate::refs::ref_list;
use crate::repository::repo_find;

pub fn cmd_show_ref() -> io::Result<()> {
    let repo = repo_find(None)?;
    let refs = ref_list(&repo, None)?;
    show_ref(refs, true, None)
}

pub fn show_ref(
    refs: IndexMap<PathBuf, String>,
    with_hash: bool,
    prefix: Option<String>,
) -> io::Result<()> {
    let mut pr = String::new();
    if let Some(p) = prefix {
        pr = p + "/";
    }

    for (k, v) in refs.iter() {
        if with_hash {
            println!("{} {}{:?}", v, pr, k);
        } else {
            println!("{}{:?}", pr, k);
        }
    }

    Ok(())
}
