use crate::cli;
use crate::index;
use crate::repository::repo_find;
use std::io;

pub fn cmd_ls_files(args: &cli::LsFilesArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let index = index::read_index(&repo)?;

    for entry in &index.entries {
        if args.stage {
            let sha = entry
                .sha
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let mode = format!("{:06o}", entry.mode);
            let stage = entry.stage();
            let path = String::from_utf8_lossy(&entry.path);
            println!("{mode} {sha} {stage}\t{path}");
        } else {
            let path = String::from_utf8_lossy(&entry.path);
            println!("{path}");
        }
    }

    Ok(())
}
