use crate::cli;
use crate::index::{self, IndexEntry};
use crate::object::{self, BlobObject, GitObject};
use crate::repository::repo_find;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub fn cmd_add(args: &cli::AddArgs) -> io::Result<()> {
    let repo = repo_find(None)?;
    let mut index = index::read_index(&repo)?;

    for file_path in &args.paths {
        let abs_path = PathBuf::from(file_path);

        let metadata = match fs::symlink_metadata(&abs_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("error: '{}': {}", file_path, e);
                continue;
            }
        };

        let mode = if metadata.is_symlink() {
            index::entry::MODE_SYMLINK
        } else if metadata.is_dir() {
            eprintln!("error: '{}': is a directory", file_path);
            continue;
        } else {
            let unix_mode = metadata.mode();
            if unix_mode & 0o111 != 0 {
                index::entry::MODE_EXECUTABLE
            } else {
                index::entry::MODE_REGULAR
            }
        };

        let data = match fs::read(&abs_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: '{}': {}", file_path, e);
                continue;
            }
        };

        let blob = GitObject::Blob(BlobObject::new(data));
        let sha_hex = object::write_object(&blob, Some(&repo))?;
        let sha_bytes: [u8; 20] = hex::decode(&sha_hex)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid SHA length"))?;

        let path_bytes = file_path.as_bytes().to_vec();
        let name_len = path_bytes.len();
        let flags = IndexEntry::make_flags(name_len, 0);

        let mut entry = IndexEntry::new(path_bytes, sha_bytes, mode, flags);
        entry.ctime_sec = metadata.ctime() as u32;
        entry.ctime_ns = metadata.ctime_nsec() as u32;
        entry.mtime_sec = metadata.mtime() as u32;
        entry.mtime_ns = metadata.mtime_nsec() as u32;
        entry.dev = metadata.dev() as u32;
        entry.ino = metadata.ino() as u32;
        entry.uid = metadata.uid();
        entry.gid = metadata.gid();
        entry.file_size = metadata.size() as u32;

        index.add_entry(entry);
    }

    index::write_index(&repo, &index)?;

    Ok(())
}
